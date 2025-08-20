use crate::cargo_toml::CargoToml;
use cargo_toml::{Dependency, DepsSet, Publish};
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk::{get_host_env_var, get_host_environment, locate_root, locate_root_with_check};
use moon_pdk_api::*;
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
    for (id, manifest) in packages.values() {
        let mut project_output = ExtendProjectOutput::default();

        let mut extract_implicit_deps =
            |package_deps: &DepsSet, scope: DependencyScope| -> AnyResult<()> {
                for (dep_name, dep) in package_deps {
                    // Only inherit if the dependency is using the local `path = "..."` syntax,
                    // and the package name exists in our gathered map
                    if dep.detail().is_some_and(|det| det.path.is_some())
                        && packages.contains_key(dep_name)
                    {
                        project_output.dependencies.push(ProjectDependency {
                            id: dep_name.into(),
                            scope,
                            via: Some(format!("crate {dep_name}")),
                        });
                    }
                }

                Ok(())
            };

        if let Some(package) = &manifest.package {
            project_output.alias = Some(package.name.clone());

            extract_implicit_deps(&manifest.dependencies, DependencyScope::Production)?;
            extract_implicit_deps(&manifest.dev_dependencies, DependencyScope::Development)?;
            extract_implicit_deps(&manifest.build_dependencies, DependencyScope::Build)?;

            output.extended_projects.insert(id.into(), project_output);

            if let Some(file) = manifest.path.virtual_path() {
                output.input_files.push(file);
            }
        }
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

    if let Some(dir) = var::get::<String>("bin_dir")? {
        paths.push(PathBuf::from(dir));
    } else {
        let maybe_dir = if let Some(value) = get_host_env_var("CARGO_INSTALL_ROOT")? {
            Some(PathBuf::from(value).join("bin"))
        } else if let Some(value) = get_host_env_var("CARGO_HOME")? {
            Some(PathBuf::from(value).join("bin"))
        } else {
            env.home_dir.join(".cargo").join("bin").real_path()
        };

        if let Some(dir) = maybe_dir {
            if let Some(dir_str) = dir.to_str() {
                var::set("bin_dir", dir_str)?;
            }

            paths.push(dir);
        }
    }

    Ok(())
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
    {
        if let Some(globals_dir) = &input.globals_dir {
            let cargo_bin_name = command.strip_prefix("cargo-").unwrap_or(command);
            let cargo_bin_path =
                globals_dir.join(env.os.get_exe_name(format!("cargo-{cargo_bin_name}")));

            // Is a cargo executable, shift over arguments
            if cargo_bin_path.exists() {
                output.command = Some("cargo".into());
                output.args = Some(Extend::Prepend(vec![cargo_bin_name.into()]));
            }
        }
    }

    // Always include Cargo specific paths for all commands
    gather_shared_paths(&env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();
    let env = get_host_environment()?;

    // Always include Cargo specific paths for all commands
    gather_shared_paths(&env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let mut output = LocateDependenciesRootOutput::default();

    // Attempt to find `Cargo.lock` first
    if let Some(root) = locate_root(&input.starting_dir, "Cargo.lock") {
        output.root = root.virtual_path();
        output.members = CargoToml::load(root.join("Cargo.toml"))?.extract_members();
    }

    // Otherwise find a `Cargo.toml` workspace
    if output.root.is_none() {
        locate_root_with_check(&input.starting_dir, "Cargo.toml", |root| {
            let manifest = CargoToml::load(root.join("Cargo.toml"))?;
            let mut found = false;

            if manifest.workspace.is_some() {
                output.root = root.virtual_path();
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
                output.root = root.virtual_path();
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
            version: Some(VersionSpec::Semantic(SemVer(package.version))),
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
                    Dependency::Simple(req) => {
                        ManifestDependency::Version(UnresolvedVersionSpec::parse(req)?)
                    }
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
                        if cfg.features.is_empty() && cfg.version.is_none() {
                            ManifestDependency::Inherited(cfg.inherited)
                        } else {
                            ManifestDependency::Config(ManifestDependencyConfig {
                                inherited: cfg.inherited,
                                features: cfg.features.clone(),
                                version: match &cfg.version {
                                    Some(version) => Some(UnresolvedVersionSpec::parse(version)?),
                                    None => None,
                                },
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

    extract_deps(&manifest.dependencies, &mut output.dependencies)?;
    extract_deps(&manifest.dev_dependencies, &mut output.dev_dependencies)?;
    extract_deps(&manifest.build_dependencies, &mut output.build_dependencies)?;

    Ok(Json(output))
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
