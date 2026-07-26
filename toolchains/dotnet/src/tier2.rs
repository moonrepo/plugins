use crate::config::DotnetToolchainConfig;
use crate::discovery::{
    LOCKFILE_SEARCH_DEPTH, contains_lockfile, find_config_files, find_lock_files,
    find_project_files, has_solution_file, walk_up,
};
use crate::eval_cache::{read_eval_cache, write_eval_cache};
use crate::msbuild::evaluate_project;
use crate::nuget_lock::parse_lock_file;
use crate::tier2_env::build_eval_env;
use extism_pdk::*;
use moon_config::{UnresolvedVersionSpec, VersionSpec};
use moon_pdk::{
    HostLogInput, HostLogTarget, command_exists, get_host_environment, host_log,
    is_project_toolchain_enabled, parse_toolchain_config,
};
use moon_pdk_api::*;
use starbase_utils::fs;
use std::collections::BTreeMap;

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let mut output = LocateDependenciesRootOutput::default();
    let workspace_root = &input.context.workspace_root;

    // Nearest solution file wins.
    for dir in walk_up(&input.starting_dir, workspace_root) {
        if has_solution_file(&dir) {
            output.root = dir.virtual_path();
            break;
        }
    }

    // Fall back to the nearest lockfile, then the nearest project file.
    for probe in [find_lock_files, find_project_files] {
        if output.root.is_some() {
            break;
        }

        for dir in walk_up(&input.starting_dir, workspace_root) {
            if !probe(&dir).is_empty() {
                output.root = dir.virtual_path();
                break;
            }
        }
    }

    // Single dependencies root for v1; no member globs.
    output.members = None;

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let config = parse_toolchain_config::<DotnetToolchainConfig>(input.toolchain_config)?;
    let mut output = InstallDependenciesOutput::default();

    let mut args: Vec<String> = vec!["restore".into()];

    // The mere presence of a lock file opts a project into lock-file restore;
    // --locked-mode additionally fails restore (NU1004) when declared
    // dependencies drifted from the lock file.
    if contains_lockfile(&input.root, LOCKFILE_SEARCH_DEPTH) {
        args.push("--locked-mode".into());
    }

    args.extend(config.restore_args.iter().cloned());

    output.install_command = Some(
        ExecCommandInput::new("dotnet", args)
            .cwd(input.root.clone())
            .into(),
    );
    // NuGet has no dedupe concept.
    output.dedupe_command = None;

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();
    let lock = parse_lock_file(&fs::read_file(&input.path)?)?;

    // Dedupe identical entries across target frameworks.
    for entries in lock.dependencies.into_values() {
        for (name, entry) in entries {
            // Project-type entries are in-repo ProjectReferences, not packages.
            if entry.dep_type.eq_ignore_ascii_case("Project") {
                continue;
            }

            let versions = output.dependencies.entry(name).or_default();

            let version = entry
                .resolved
                .as_deref()
                .and_then(|value| VersionSpec::parse(value).ok());

            let already_present = versions.iter().any(|existing: &LockDependency| {
                existing.version == version && existing.hash == entry.content_hash
            });

            if !already_present {
                versions.push(LockDependency {
                    hash: entry.content_hash,
                    meta: None,
                    // NuGet ranges like "[13.0.3, )" may not parse; omit then.
                    req: entry
                        .requested
                        .as_deref()
                        .and_then(|value| UnresolvedVersionSpec::parse(value).ok()),
                    version,
                });
            }
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();

    let Some(real_path) = input.path.real_path() else {
        return Ok(Json(output));
    };

    let env = get_host_environment()?;

    // Degrade silently like hash_task_contents: a missing dotnet must not
    // fail moon's install fingerprinting.
    if !command_exists(&env, "dotnet") {
        return Ok(Json(output));
    }

    let manifest_dir = input
        .path
        .parent()
        .unwrap_or_else(|| input.context.workspace_root.clone());

    // `parse_manifest` carries no toolchain config, so an explicit
    // `dotnetRoot` cannot be honored here; the env var and the guarded
    // `~/.dotnet` fallback still apply.
    let eval_env = build_eval_env(
        &DotnetToolchainConfig::default(),
        manifest_dir,
        &input.context.workspace_root,
    )?;

    let evaluation = match evaluate_project(&real_path, &eval_env) {
        Ok(evaluation) => evaluation,
        Err(error) => {
            host_log!(
                warn,
                "MSBuild evaluation failed while parsing manifest {}: {}",
                real_path.display(),
                error
            );

            return Ok(Json(output));
        }
    };

    // NuGet range syntax ("[13.0.3]", "(1.0,2.0)") is not a moon version
    // spec; keep the raw string as a reference so the dependency is still
    // listed (it just won't contribute a version to fingerprints).
    let to_dependency = |version: String| match UnresolvedVersionSpec::parse(&version) {
        Ok(spec) => ManifestDependency::new(spec),
        Err(_) => ManifestDependency::Config(ManifestDependencyConfig {
            reference: Some(version),
            ..Default::default()
        }),
    };

    let is_packages_props = input
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("Directory.Packages.props"));

    if is_packages_props {
        // Central Package Management: PackageVersion items declare the
        // workspace-level versions that versionless PackageReferences
        // inherit. This is the only manifest name moon can actually track
        // for .NET — project files have variable names, which moon's
        // literal-name manifest matching cannot express.
        for (name, version) in evaluation.package_versions() {
            output.dependencies.insert(name, to_dependency(version));
        }
    } else {
        for (name, version) in evaluation.package_references() {
            let dep = if version == "*" {
                // Versionless under CPM: inherited from the workspace
                // manifest (Directory.Packages.props).
                ManifestDependency::inherited()
            } else {
                to_dependency(version)
            };

            output.dependencies.insert(name, dep);
        }

        output.publishable = evaluation
            .property("IsPackable")
            .eq_ignore_ascii_case("true");
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn hash_task_contents(
    Json(input): Json<HashTaskContentsInput>,
) -> FnResult<Json<HashTaskContentsOutput>> {
    let mut output = HashTaskContentsOutput::default();

    if !is_project_toolchain_enabled(&input.project) {
        return Ok(Json(output));
    }

    let project_root = input.context.get_project_root(&input.project);

    // Config files (Directory.Build.props/targets/rsp, Directory.Packages.props,
    // nuget.config, global.json) from the project dir up to the workspace root
    // are always hashed: conditions/imports can make any of them affect the
    // resolved package set, and props/targets/rsp change build behavior even
    // when the package set is fully pinned by a lock file. Effects of custom
    // `<Import>`s outside these conventions are only captured via the
    // evaluated package set below, not content-hashed.
    let mut configs: BTreeMap<String, String> = BTreeMap::new();
    let workspace_root = &input.context.workspace_root;

    for dir in walk_up(&project_root, workspace_root) {
        for file in find_config_files(&dir) {
            let key = file
                .virtual_path()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| file.to_string());

            configs.insert(key, fs::read_file(&file)?);
        }
    }

    // Lock file(s) present: their content already pins the entire resolved
    // package set (incl. contentHashes) — include them raw and skip the
    // costly MSBuild evaluation.
    let lock_files = find_lock_files(&project_root);

    if !lock_files.is_empty() {
        let mut lockfiles: BTreeMap<String, String> = BTreeMap::new();

        for file in &lock_files {
            let key = file
                .virtual_path()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_else(|| file.to_string());

            lockfiles.insert(key, fs::read_file(file)?);
        }

        output.contents.push(json::json!({
            "configs": configs,
            "lockfiles": lockfiles,
        }));

        return Ok(Json(output));
    }

    // No lock file: hash the *evaluated* PackageReference set instead.
    //
    // Three levels of reuse, because this function runs once per task and a
    // cold MSBuild evaluation costs ~0.5s per project:
    //   1. a plugin-instance var, for repeated tasks of the same project;
    //   2. the on-disk cache the batched graph evaluation primed, which is
    //      what keeps a lock-file-less workspace from paying one evaluation
    //      per project here (the batch already evaluated them all at once);
    //   3. evaluating this project alone.
    let cache_key = format!("eval-packages:{}", input.project.id);

    let packages: BTreeMap<String, String> = if let Some(cached) = var::get::<String>(&cache_key)? {
        serde_json::from_str(&cached)?
    } else if let Some(cached) =
        read_eval_cache(workspace_root, input.project.id.as_str(), &project_root)
    {
        var::set(&cache_key, serde_json::to_string(&cached)?)?;

        cached
    } else {
        let mut packages = BTreeMap::new();
        let mut evaluated_all = false;
        let env = get_host_environment()?;

        if command_exists(&env, "dotnet") {
            let config = parse_toolchain_config::<DotnetToolchainConfig>(input.toolchain_config)?;
            let eval_env = build_eval_env(&config, project_root.clone(), workspace_root)?;

            evaluated_all = true;

            for file in find_project_files(&project_root) {
                let Some(real_path) = file.real_path() else {
                    evaluated_all = false;
                    continue;
                };

                match evaluate_project(&real_path, &eval_env) {
                    Ok(evaluation) => {
                        packages.extend(evaluation.package_references());
                    }
                    Err(error) => {
                        host_log!(
                            warn,
                            "MSBuild evaluation failed while hashing <id>{}</id>: {}",
                            input.project.id,
                            error
                        );

                        evaluated_all = false;
                    }
                }
            }
        }

        // Kept regardless: the var is scoped to this plugin instance, so it
        // stops us re-evaluating once per task while an SDK is genuinely
        // missing, and it disappears with the process.
        var::set(&cache_key, serde_json::to_string(&packages)?)?;

        // The on-disk cache only ever holds a complete set. Writing a partial
        // one would persist it under a digest that keeps validating, and since
        // this set is the only hash signal for a workspace without lock files,
        // package changes would stop invalidating task hashes — moon would
        // serve stale builds, and installing the missing SDK later would not
        // recover it.
        if evaluated_all {
            write_eval_cache(
                workspace_root,
                input.project.id.as_str(),
                &project_root,
                packages.clone(),
            );
        }

        packages
    };

    output.contents.push(json::json!({
        "configs": configs,
        "packages": packages,
    }));

    Ok(Json(output))
}
