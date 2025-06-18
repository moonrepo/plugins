use crate::config::GoToolchainConfig;
use crate::go_mod::parse_go_mod;
use crate::go_sum::GoSum;
use crate::go_work::GoWork;
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk::{get_host_env_var, get_host_environment, parse_toolchain_config};
use moon_pdk_api::*;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let mut output = ExtendProjectGraphOutput::default();

    // First pass, gather all packages and their manifests
    let mut packages = BTreeMap::default();

    for (id, source) in input.project_sources {
        let go_mod_path = input.context.workspace_root.join(source).join("go.mod");

        if go_mod_path.exists() {
            let manifest = parse_go_mod(fs::read_file(&go_mod_path)?)?;

            packages.insert(manifest.module.clone(), (id, manifest));

            if let Some(file) = go_mod_path.virtual_path() {
                output.input_files.push(file);
            }
        }
    }

    // Second pass, extract packages and their relationships
    for (id, manifest) in packages.values() {
        let mut project_output = ExtendProjectOutput {
            alias: Some(manifest.module.clone()),
            ..Default::default()
        };

        for dep in &manifest.require {
            if packages.contains_key(&dep.module.module_path) {
                project_output.dependencies.push(ProjectDependency {
                    id: dep.module.module_path.clone(),
                    scope: DependencyScope::Production,
                    via: Some(format!("module {}", dep.module.module_path)),
                });
            }
        }

        output.extended_projects.insert(id.into(), project_output);
    }

    Ok(Json(output))
}

fn gather_shared_paths(
    env: &HostEnvironment,
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(globals_dir) = globals_dir {
        if let Some(value) = globals_dir.real_path() {
            paths.push(value);

            // Avoid the host env overhead if we already
            // have a valid globals directory!
            return Ok(());
        }
    }

    if let Some(value) = get_host_env_var("GOBIN")? {
        paths.push(PathBuf::from(value));
    } else if let Some(value) = get_host_env_var("GOPATH")? {
        paths.push(PathBuf::from(value).join("bin"));
    } else if let Some(value) = env.home_dir.join("go/bin").real_path() {
        paths.push(value);
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let env = get_host_environment()?;

    // Always include Go specific paths for all commands
    gather_shared_paths(&env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();
    let env = get_host_environment()?;

    // Always include Go specific paths for all commands
    gather_shared_paths(&env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let mut output = LocateDependenciesRootOutput::default();

    let locate = |starting_dir: &VirtualPath, file: &str| -> Option<VirtualPath> {
        let mut current_dir = Some(starting_dir.to_owned());

        while let Some(dir) = current_dir {
            if dir.join(file).exists() {
                return Some(dir);
            }

            current_dir = dir.parent();
        }

        None
    };

    // Find `go.work` first
    if let Some(root) = locate(&input.starting_dir, "go.work") {
        let go_work = GoWork::parse(fs::read_file(root.join("go.work"))?)?;

        if !go_work.modules.is_empty() {
            output.members = Some(go_work.modules);
        }

        output.root = root.virtual_path();
    }

    // Then `go.sum` second
    if output.root.is_none() {
        if let Some(root) = locate(&input.starting_dir, "go.sum") {
            output.root = root.virtual_path();
        }
    }

    // Otherwise assume `go.mod`
    if output.root.is_none() {
        if let Some(root) = locate(&input.starting_dir, "go.mod") {
            output.root = root.virtual_path();
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let mut output = InstallDependenciesOutput::default();
    let config = parse_toolchain_config::<GoToolchainConfig>(input.toolchain_config)?;

    if input.root.join("go.work").exists() {
        output.install_command = Some(
            ExecCommandInput::new("go", ["work", "sync"])
                .cwd(input.root.clone())
                .into(),
        );
    } else if input.root.join("go.mod").exists() {
        output.install_command = Some(
            ExecCommandInput::new("go", ["mod", "download"])
                .cwd(input.root.clone())
                .into(),
        );

        if input.root.join("go.sum").exists() && config.tidy_on_change {
            output.dedupe_command = Some(
                ExecCommandInput::new("go", ["mod", "tidy"])
                    .cwd(input.root)
                    .into(),
            );
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();
    let go_sum = GoSum::parse(fs::read_file(input.path)?)?;

    for (module, entry) in go_sum.dependencies {
        output
            .dependencies
            .entry(module)
            .or_default()
            .push(LockDependency {
                hash: entry
                    .checksum
                    .strip_prefix("h1:") // sha256
                    .map(|hash| hash.to_owned()),
                version: Some(entry.version),
                ..Default::default()
            });
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();
    let go_mod = parse_go_mod(fs::read_file(input.path)?)?;

    for dep in go_mod.require {
        // Ignore transitive deps, as we only care about
        // direct project deps during task hashing
        if dep.indirect {
            continue;
        }

        output.dependencies.insert(
            dep.module.module_path,
            ManifestDependency::Version(UnresolvedVersionSpec::parse(dep.module.version)?),
        );
    }

    Ok(Json(output))
}
