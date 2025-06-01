use crate::cargo_toml::CargoToml;
use cargo_toml::{Dependency, DepsSet, Publish};
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk::{get_host_env_var, get_host_environment};
use moon_pdk_api::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let mut output = ExtendProjectGraphOutput::default();

    for (id, source) in input.project_sources {
        let cargo_toml_path = input.context.workspace_root.join(source).join("Cargo.toml");
        let mut project_output = ExtendProjectOutput::default();

        let mut extract_implicit_deps =
            |package_deps: &DepsSet, scope: DependencyScope| -> AnyResult<()> {
                for (dep_name, dep) in package_deps {
                    // Only inherit if the dependency is using the local `path = "..."` syntax
                    if dep.detail().is_some_and(|det| det.path.is_some()) {
                        project_output.dependencies.push(ProjectDependency {
                            id: dep_name.into(),
                            scope,
                            via: Some(format!("crate {dep_name}")),
                        });
                    }
                }

                Ok(())
            };

        if cargo_toml_path.exists() {
            let cargo = CargoToml::load(cargo_toml_path.clone())?;

            if let Some(package) = &cargo.package {
                project_output.alias = Some(package.name.clone());

                extract_implicit_deps(&cargo.dependencies, DependencyScope::Production)?;
                extract_implicit_deps(&cargo.dev_dependencies, DependencyScope::Development)?;
                extract_implicit_deps(&cargo.build_dependencies, DependencyScope::Build)?;

                output.extended_projects.insert(id, project_output);

                if let Some(file) = cargo_toml_path.virtual_path() {
                    output.input_files.push(file);
                }
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

    if let Some(value) = get_host_env_var("CARGO_INSTALL_ROOT")? {
        paths.push(PathBuf::from(value).join("bin"));
    } else if let Some(value) = get_host_env_var("CARGO_HOME")? {
        paths.push(PathBuf::from(value).join("bin"));
    } else if let Some(value) = env.home_dir.join(".cargo/bin").real_path() {
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
    let mut current_dir = Some(input.starting_dir.clone());

    while let Some(dir) = &current_dir {
        if dir.join("Cargo.lock").exists() {
            output.root = dir.virtual_path();

            let manifest_path = dir.join("Cargo.toml");

            if manifest_path.exists() {
                output.members = CargoToml::load(manifest_path)?.extract_members();
            }

            break;
        }

        current_dir = dir.parent();
    }

    // Otherwise find a `Cargo.toml` workspace
    if output.root.is_none() {
        let mut current_dir = Some(input.starting_dir.clone());

        while let Some(dir) = &current_dir {
            let manifest_path = dir.join("Cargo.toml");

            if manifest_path.exists() {
                let manifest = CargoToml::load(manifest_path)?;

                if manifest.workspace.is_some() {
                    output.root = dir.virtual_path();
                    output.members = manifest.extract_members();
                    break;
                }
            }

            current_dir = dir.parent();
        }
    }

    // Else may be a stand-alone project
    if output.root.is_none() {
        let mut current_dir = Some(input.starting_dir.clone());

        while let Some(dir) = &current_dir {
            let manifest_path = dir.join("Cargo.toml");

            if manifest_path.exists() {
                let manifest = CargoToml::load(manifest_path)?;

                if manifest.package.is_some() {
                    output.root = dir.virtual_path();
                    break;
                }
            }

            current_dir = dir.parent();
        }
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
        let mut cmd = ExecCommandInput::new("cargo", ["generate-lockfile"]);
        cmd.working_dir = Some(input.root);

        output.install_command = Some(cmd.into());
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
                            ManifestDependency::Config {
                                inherited: true,
                                features: cfg.features.clone(),
                                version: None,
                            }
                        }
                    }
                    Dependency::Detailed(cfg) => {
                        if cfg.features.is_empty() && cfg.version.is_none() {
                            ManifestDependency::Inherited(cfg.inherited)
                        } else {
                            ManifestDependency::Config {
                                inherited: cfg.inherited,
                                features: cfg.features.clone(),
                                version: match &cfg.version {
                                    Some(version) => Some(UnresolvedVersionSpec::parse(version)?),
                                    None => None,
                                },
                            }
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
