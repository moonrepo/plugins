use crate::cargo_toml::CargoToml;
use crate::config::RustToolchainConfig;
use crate::version::to_channel;
use cargo_toml::{Dependency, DepsSet, Publish};
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk::{
    VirtualPathExt, command_exists, get_host_env_var, get_host_environment, locate_root,
    locate_root_with_check, parse_toolchain_config,
};
use moon_pdk_api::*;
use rustc_hash::FxHashMap;
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
        let cargo_toml_path = input.context.workspace_root.join(source).join("Cargo.toml");

        if cargo_toml_path.exists() {
            let manifest = CargoToml::load(cargo_toml_path.clone())?;

            if let Some(package) = &manifest.package {
                packages.insert(package.name().to_owned(), (id, manifest));
            }
        }
    }

    // Second pass, extract packages and their relationships
    let mut workspaces = FxHashMap::default();

    for (id, manifest) in packages.values() {
        let Some(package) = &manifest.package else {
            continue;
        };

        let mut project_output = ExtendProjectOutput {
            alias: Some(package.name.clone()),
            ..Default::default()
        };

        let mut deps_sets = vec![
            (&manifest.dependencies, DependencyScope::Production),
            (&manifest.dev_dependencies, DependencyScope::Development),
            (&manifest.build_dependencies, DependencyScope::Build),
        ];

        for target in manifest.target.values() {
            deps_sets.push((&target.dependencies, DependencyScope::Production));
            deps_sets.push((&target.dev_dependencies, DependencyScope::Development));
            deps_sets.push((&target.build_dependencies, DependencyScope::Build));
        }

        // `workspace = true` dependencies are defined in the workspace root
        let workspace_deps = if deps_sets.iter().any(|(deps, _)| {
            deps.values()
                .any(|dep| matches!(dep, Dependency::Inherited(_)))
        }) {
            locate_workspace_deps(manifest, &mut workspaces)?
        } else {
            None
        };

        if let Some((root_manifest_path, _)) = &workspace_deps
            && !output.input_files.contains(root_manifest_path)
        {
            output.input_files.push(root_manifest_path.to_owned());
        }

        for (deps, scope) in deps_sets {
            for (dep_name, dep) in deps {
                let dep = match dep {
                    Dependency::Inherited(_) => {
                        match workspace_deps
                            .as_ref()
                            .and_then(|(_, deps)| deps.get(dep_name))
                        {
                            Some(dep) => dep,
                            None => continue,
                        }
                    }
                    _ => dep,
                };

                // Only inherit if the dependency is using the local `path = "..."` syntax,
                // and the package (which may be renamed) exists in our gathered map
                let Some(detail) = dep.detail().filter(|detail| detail.path.is_some()) else {
                    continue;
                };

                let package_name = detail.package.as_deref().unwrap_or(dep_name);

                let Some((dep_id, _)) = packages.get(package_name) else {
                    continue;
                };

                // Skip self-dependencies and duplicates across target tables
                if dep_id == id
                    || project_output
                        .dependencies
                        .iter()
                        .any(|existing| &existing.id == dep_id && existing.scope == scope)
                {
                    continue;
                }

                project_output.dependencies.push(ProjectDependency {
                    id: dep_id.to_owned(),
                    scope,
                    via: Some(format!("crate {package_name}")),
                });
            }
        }

        output
            .extended_projects
            .insert(id.to_owned(), project_output);

        output.input_files.push(manifest.path.clone());
    }

    Ok(Json(output))
}

/// Find the `[workspace.dependencies]` of the Cargo workspace that the manifest
/// belongs to, by traversing upwards. Roots are cached as members share them.
fn locate_workspace_deps<'cache>(
    manifest: &CargoToml,
    cache: &'cache mut FxHashMap<VirtualPath, DepsSet>,
) -> AnyResult<Option<(VirtualPath, &'cache DepsSet)>> {
    let Some(starting_dir) = manifest.path.parent() else {
        return Ok(None);
    };

    let mut root_manifest_path = None;

    locate_root_with_check(&starting_dir, "Cargo.toml", |dir| {
        let manifest_path = dir.join("Cargo.toml");

        if !cache.contains_key(&manifest_path) {
            let Some(workspace) = CargoToml::load(manifest_path.clone())?.workspace.take() else {
                return Ok(false);
            };

            cache.insert(manifest_path.clone(), workspace.dependencies);
        }

        root_manifest_path = Some(manifest_path);

        Ok(true)
    })?;

    Ok(root_manifest_path.and_then(|path| cache.get(&path).map(|deps| (path, deps))))
}

fn gather_shared_paths(
    env: &HostEnvironment,
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(globals_dir) = globals_dir
        && globals_dir.to_real_path()?.is_some()
    {
        // Avoid the host env overhead if we already
        // have a valid globals directory!
        return Ok(());
    }

    if let Some(dirs) = var::get::<String>("bin_dirs")? {
        paths.extend(dirs.lines().map(PathBuf::from));

        return Ok(());
    }

    let mut dirs = vec![];

    // Binaries from `cargo install` are written to the install root,
    // while the rustup proxies (`cargo`, `rustc`, etc) remain in the home
    if let Some(value) = get_host_env_var("CARGO_INSTALL_ROOT")? {
        dirs.push(PathBuf::from(value).join("bin"));
    }

    if let Some(value) = get_host_env_var("CARGO_HOME")? {
        dirs.push(PathBuf::from(value).join("bin"));
    } else if let Some(dir) = env.home_dir.join(".cargo").join("bin").to_real_path()? {
        dirs.push(dir.to_path_buf());
    }

    var::set(
        "bin_dirs",
        dirs.iter()
            .filter_map(|dir| dir.to_str())
            .collect::<Vec<_>>()
            .join("\n"),
    )?;

    paths.extend(dirs);

    Ok(())
}

/// The channel to pin with `RUSTUP_TOOLCHAIN`, as rustup ignores moon's
/// `PROTO_RUST_VERSION`. Partial versions are skipped (rustup treats them as
/// another toolchain), and a `RUSTUP_TOOLCHAIN` set on the host still wins.
pub(crate) fn get_pinned_toolchain(config: &RustToolchainConfig) -> AnyResult<Option<String>> {
    let Some(channel) = config.version.as_ref().and_then(to_channel) else {
        return Ok(None);
    };

    if get_host_env_var("RUSTUP_TOOLCHAIN")?.is_some_and(|value| !value.is_empty()) {
        return Ok(None);
    }

    Ok(Some(channel))
}

// Bare commands may be binaries of their own, like `make` instead of
// `cargo make`, or `fmt` instead of rustup's `cargo fmt` proxy
fn is_command_installed(
    env: &HostEnvironment,
    globals_dir: &VirtualPath,
    command: &str,
) -> AnyResult<bool> {
    if globals_dir.join(env.os.get_exe_name(command)).exists() {
        return Ok(true);
    }

    // Checking the `PATH` spawns a process, so cache it
    let key = format!("installed:{command}");

    if let Some(value) = var::get::<String>(&key)? {
        return Ok(value == "true");
    }

    let installed = command_exists(env, command);

    var::set(&key, if installed { "true" } else { "false" })?;

    Ok(installed)
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let env = get_host_environment()?;
    let command = &input.command;

    // Binary may be installed to `~/.cargo/bin` so we must prefix
    // it with `cargo` so that it can actually execute...
    if command != "cargo" &&
        command != "rls" &&
        // rustc, rustdoc, etc
        !command.starts_with("rust")
        && let Some(globals_dir) = &input.globals_dir
    {
        let cargo_bin_name = command.strip_prefix("cargo-").unwrap_or(command);
        let cargo_bin_path =
            globals_dir.join(env.os.get_exe_name(format!("cargo-{cargo_bin_name}")));

        // Is a cargo executable, shift over arguments
        if cargo_bin_path.exists()
            && (command.starts_with("cargo-") || !is_command_installed(env, globals_dir, command)?)
        {
            output.command = Some("cargo".into());
            output.args = Some(Extend::Prepend(vec![cargo_bin_name.into()]));
        }
    }

    let config = parse_toolchain_config::<RustToolchainConfig>(input.toolchain_config)?;

    if let Some(channel) = get_pinned_toolchain(&config)? {
        output.env.insert("RUSTUP_TOOLCHAIN".into(), channel);
    }

    // Always include Cargo specific paths for all commands
    gather_shared_paths(env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();
    let env = get_host_environment()?;
    let config = parse_toolchain_config::<RustToolchainConfig>(input.toolchain_config)?;

    if let Some(channel) = get_pinned_toolchain(&config)? {
        output.env.insert("RUSTUP_TOOLCHAIN".into(), channel);
    }

    // Always include Cargo specific paths for all commands
    gather_shared_paths(env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let mut output = LocateDependenciesRootOutput::default();

    // Attempt to find `Cargo.lock` first
    if let Some(root) = locate_root(&input.starting_dir, "Cargo.lock") {
        output.members = CargoToml::load(root.join("Cargo.toml"))?.extract_members();
        output.root = Some(root);
    }

    // Otherwise find a `Cargo.toml` workspace
    if output.root.is_none() {
        locate_root_with_check(&input.starting_dir, "Cargo.toml", |root| {
            let manifest = CargoToml::load(root.join("Cargo.toml"))?;
            let mut found = false;

            if manifest.workspace.is_some() {
                output.root = Some(root.to_owned());
                output.members = manifest.extract_members();
                found = true;
            }

            Ok(found)
        })?;
    }

    // Else may be a stand-alone project
    if output.root.is_none() {
        locate_root_with_check(&input.starting_dir, "Cargo.toml", |root| {
            let manifest = CargoToml::load(root.join("Cargo.toml"))?;
            let mut found = false;

            if manifest.package.is_some() {
                output.root = Some(root.to_owned());
                found = true;
            }

            Ok(found)
        })?;
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let mut output = InstallDependenciesOutput::default();

    // Cargo does not support an "install dependencies" command
    // as it automatically happens when running any Cargo commands.
    // However, if we don't detect a lockfile, we can attempt
    // to generate one!
    if !input.root.join("Cargo.lock").exists() {
        output.install_command = Some(
            ExecCommandInput::new("cargo", ["generate-lockfile"])
                .cwd(input.root)
                .into(),
        );
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();
    let lock = cargo_lock::Lockfile::load(input.path)?;

    for package in lock.packages {
        let mut dep = LockDependency {
            version: VersionSpec::parse(package.version.to_string()).ok(),
            ..Default::default()
        };

        if let Some(checksum) = package.checksum {
            dep.hash = Some(checksum.to_string());
        }

        output
            .dependencies
            .entry(package.name.as_str().to_owned())
            .or_default()
            .push(dep);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();
    let manifest = CargoToml::load(input.path)?;

    let extract_deps = |in_deps: &BTreeMap<String, Dependency>,
                        out_deps: &mut BTreeMap<String, ManifestDependency>|
     -> AnyResult<()> {
        for (name, in_dep) in in_deps {
            out_deps.insert(
                name.to_owned(),
                match in_dep {
                    Dependency::Simple(req) => match parse_requirement(req) {
                        Some(version) => ManifestDependency::Version(version),
                        None => continue,
                    },
                    Dependency::Inherited(cfg) => {
                        if cfg.features.is_empty() {
                            ManifestDependency::Inherited(true)
                        } else {
                            ManifestDependency::Config(ManifestDependencyConfig {
                                inherited: true,
                                features: cfg.features.clone(),
                                ..Default::default()
                            })
                        }
                    }
                    Dependency::Detailed(cfg) => {
                        if cfg.features.is_empty()
                            && cfg.version.is_none()
                            && cfg.path.is_none()
                            && cfg.git.is_none()
                        {
                            ManifestDependency::Inherited(cfg.inherited)
                        } else {
                            ManifestDependency::Config(ManifestDependencyConfig {
                                inherited: cfg.inherited,
                                features: cfg.features.clone(),
                                path: cfg.path.as_ref().map(PathBuf::from),
                                url: cfg.git.clone(),
                                version: cfg.version.as_deref().and_then(parse_requirement),
                                ..Default::default()
                            })
                        }
                    }
                },
            );
        }

        Ok(())
    };

    if let Some(package) = &manifest.package {
        if let Ok(version) = package.version.get() {
            output.version = Some(Version::parse(version)?);
        }

        if let Ok(publish) = package.publish.get() {
            output.publishable = match publish {
                Publish::Flag(state) => *state,
                Publish::Registry(list) => !list.is_empty(),
            };
        }
    }

    if let Some(workspace) = &manifest.workspace {
        extract_deps(&workspace.dependencies, &mut output.dependencies)?;
    }

    // Extracted before the main tables, so those take precedence
    for target in manifest.target.values() {
        extract_deps(&target.dependencies, &mut output.dependencies)?;
        extract_deps(&target.dev_dependencies, &mut output.dev_dependencies)?;
        extract_deps(&target.build_dependencies, &mut output.build_dependencies)?;
    }

    extract_deps(&manifest.dependencies, &mut output.dependencies)?;
    extract_deps(&manifest.dev_dependencies, &mut output.dev_dependencies)?;
    extract_deps(&manifest.build_dependencies, &mut output.build_dependencies)?;

    Ok(Json(output))
}

// Cargo treats bare requirements (`1.2.3`) as caret, so they can match the
// resolved version in the lockfile. Unsupported syntax is skipped.
fn parse_requirement(req: &str) -> Option<UnresolvedVersionSpec> {
    let req = req.trim();

    if req.starts_with(|ch: char| ch.is_ascii_digit()) {
        UnresolvedVersionSpec::parse(format!("^{req}")).ok()
    } else {
        UnresolvedVersionSpec::parse(req).ok()
    }
}

#[plugin_fn]
pub fn hash_task_contents(
    Json(_): Json<HashTaskContentsInput>,
) -> FnResult<Json<HashTaskContentsOutput>> {
    let env = get_host_environment()?;

    let mut map = json::Map::default();
    map.insert("os".into(), json::Value::String(env.os.to_string()));
    map.insert("arch".into(), json::Value::String(env.arch.to_string()));
    map.insert("libc".into(), json::Value::String(env.libc.to_string()));

    Ok(Json(HashTaskContentsOutput {
        contents: vec![json::Value::Object(map)],
    }))
}
