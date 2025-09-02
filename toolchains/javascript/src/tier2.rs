use crate::config::*;
use crate::infer_tasks::TasksInferrer;
use crate::lockfiles::*;
use crate::package_json::PackageJson;
use extism_pdk::*;
use moon_common::path::paths_are_equal;
use moon_config::DependencyScope;
use moon_pdk::{
    get_host_environment, load_toolchain_config, locate_root_many, locate_root_many_with_check,
    parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use nodejs_package_json::{VersionProtocol, WorkspaceProtocol};
use starbase_utils::{json as jsonc, yaml};
use std::collections::BTreeMap;
use std::path::PathBuf;

// TODO deno
#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendProjectGraphOutput::default();

    // First pass, gather all packages and their manifests
    let mut packages = BTreeMap::default();

    for (id, source) in input.project_sources {
        let project_root = input.context.get_project_root_from_source(&source);
        let package_path = project_root.join("package.json");

        if package_path.exists() {
            let manifest = PackageJson::load(package_path)?;

            if let Some(name) = &manifest.name {
                packages.insert(name.to_owned(), (id, project_root, manifest));
            }
        }
    }

    // Second pass, extract packages and their relationships
    for (id, project_root, manifest) in packages.values() {
        let mut project_output = ExtendProjectOutput::default();

        let mut extract_implicit_deps =
            |package_deps: &Option<BTreeMap<String, VersionProtocol>>,
             scope: DependencyScope|
             -> AnyResult<()> {
                let Some(deps) = package_deps else {
                    return Ok(());
                };

                for (dep_name, dep_version) in deps {
                    let Some((dep_id, dep_root, _)) = packages.get(dep_name) else {
                        continue;
                    };

                    // Only inherit if the dependency is in the local workspace
                    let is_local = match dep_version {
                        VersionProtocol::File(rel_path)
                        | VersionProtocol::Link(rel_path)
                        | VersionProtocol::Portal(rel_path) => {
                            paths_are_equal(dep_root, project_root.join(rel_path))
                        }
                        VersionProtocol::Workspace(_) => true,
                        _ => false,
                    };

                    if is_local {
                        project_output.dependencies.push(ProjectDependency {
                            id: dep_id.into(),
                            scope,
                            via: Some(format!("package {dep_name}")),
                        });
                    }
                }

                Ok(())
            };

        if let Some(name) = &manifest.name {
            project_output.alias = Some(name.to_owned());
        }

        extract_implicit_deps(&manifest.dependencies, DependencyScope::Production)?;
        extract_implicit_deps(&manifest.dev_dependencies, DependencyScope::Development)?;
        extract_implicit_deps(&manifest.peer_dependencies, DependencyScope::Peer)?;
        extract_implicit_deps(&manifest.optional_dependencies, DependencyScope::Build)?;

        if config.infer_tasks_from_scripts {
            project_output.tasks = TasksInferrer::new(&config, manifest).infer()?;
        }

        output.extended_projects.insert(id.into(), project_output);

        if let Some(file) = manifest.path.virtual_path() {
            output.input_files.push(file);
        }
    }

    Ok(Json(output))
}

fn gather_shared_paths(
    context: &MoonContext,
    project: &ProjectFragment,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    // Local packages upwards to the root
    let mut current_dir = context.get_project_root(project);

    while current_dir != context.workspace_root {
        if let Some(bin_dir) = current_dir.join("node_modules").join(".bin").real_path() {
            paths.push(bin_dir);
        }

        match current_dir.parent() {
            Some(dir) => {
                current_dir = dir;
            }
            None => {
                break;
            }
        }
    }

    if let Some(bin_dir) = context
        .workspace_root
        .join("node_modules")
        .join(".bin")
        .real_path()
    {
        paths.push(bin_dir);
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();

    gather_shared_paths(&input.context, &input.project, &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    gather_shared_paths(&input.context, &input.project, &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn define_requirements(
    Json(input): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config)?;
    let mut output = DefineRequirementsOutput::default();

    if let Some(package_manager) = config.package_manager {
        if package_manager.is_for_node() {
            output.requires.push("unstable_node".into());
        }

        output.requires.push(format!("unstable_{package_manager}"));
    }

    Ok(Json(output))
}

fn extract_workspace_members(
    package_manager: JavaScriptPackageManager,
    root: &VirtualPath,
) -> AnyResult<Option<Vec<String>>> {
    let cache_key = format!("workspace-members:{root}");
    let mut members = None;

    // Reduce the amount of file system operations for every package
    // within the workspace by caching the found members for this directory
    if let Some(cache) = var::get::<String>(&cache_key)? {
        let members: Vec<String> = json::from_str(&cache)?;

        return Ok(Some(members));
    }

    // Package manager specific files
    match package_manager {
        JavaScriptPackageManager::Deno => {
            let config_file = root.join("deno.json");
            let configc_file = root.join("deno.jsonc");

            let config: DenoJson = if config_file.exists() {
                jsonc::read_file(config_file)?
            } else if configc_file.exists() {
                jsonc::read_file(configc_file)?
            } else {
                Default::default()
            };

            members = config.workspace.map(|ws| ws.get_members().to_vec());
        }
        JavaScriptPackageManager::Pnpm => {
            let workspace_file = root.join("pnpm-workspace.yaml");

            if workspace_file.exists() {
                let workspace: PnpmWorkspace = yaml::read_file(workspace_file)?;

                members = workspace.packages;
            }
        }
        _ => {}
    };

    // Otherwise `package.json` itself
    if members.is_none() {
        members = PackageJson::load(root.join("package.json"))?.extract_members();
    }

    // Cache the result!
    if let Some(members) = &members {
        var::set(cache_key, json::to_string(members)?)?;
    }

    Ok(members)
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config)?;
    let mut output = LocateDependenciesRootOutput::default();

    let Some(package_manager) = config.package_manager else {
        return Ok(Json(output));
    };

    let manifest_names = match package_manager {
        JavaScriptPackageManager::Deno => vec!["deno.json", "deno.jsonc", "package.json"],
        _ => vec!["package.json"],
    };

    let workspace_manifest_names = match package_manager {
        JavaScriptPackageManager::Deno => vec!["deno.json", "deno.jsonc", "package.json"],
        JavaScriptPackageManager::Pnpm => vec!["pnpm-workspace.yaml", "package.json"],
        _ => vec!["package.json"],
    };

    let lock_names = match package_manager {
        JavaScriptPackageManager::Bun => vec!["bun.lock", "bun.lockb"],
        JavaScriptPackageManager::Deno => vec!["deno.lock"],
        JavaScriptPackageManager::Npm => vec!["package-lock.json", "npm-shrinkwrap.json"],
        JavaScriptPackageManager::Pnpm => vec!["pnpm-lock.yaml"],
        JavaScriptPackageManager::Yarn => vec!["yarn.lock"],
    };

    // First attempt: find lock files
    if let Some(root) = locate_root_many(&input.starting_dir, &lock_names) {
        output.root = root.virtual_path();
        output.members = extract_workspace_members(package_manager, &root)?;
    }

    // Second attempt: find workspace-compatible manifest files
    if output.root.is_none() {
        locate_root_many_with_check(&input.starting_dir, &workspace_manifest_names, |root| {
            let mut found = false;

            if let Some(members) = extract_workspace_members(package_manager, root)? {
                output.root = root.virtual_path();
                output.members = Some(members);
                found = true;
            }

            Ok(found)
        })?;
    }

    // Last attempt: find a manifest file (project only)
    if output.root.is_none()
        && let Some(root) = locate_root_many(&input.starting_dir, &manifest_names)
    {
        output.root = root.virtual_path();
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config)?;
    let mut output = InstallDependenciesOutput::default();

    let Some(package_manager) = config.package_manager else {
        return Ok(Json(output));
    };

    let env = get_host_environment()?;
    let package_manager_config =
        load_toolchain_config::<SharedPackageManagerConfig>(package_manager.to_string())?;
    let mut inherit_install_args = true;

    // Install
    let mut command = match package_manager {
        JavaScriptPackageManager::Bun => {
            let mut cmd = ExecCommandInput::new("bun", ["install"]);

            if input.production {
                cmd.args.push("--production".into());
            }

            for package_name in input.packages {
                cmd.args.push("--filter".into());
                cmd.args.push(package_name);
            }

            cmd
        }
        JavaScriptPackageManager::Deno => {
            let cmd = ExecCommandInput::new("deno", ["install"]);

            // if input.production {
            //     cmd.args.push("--production".into());
            // }

            // for package_name in input.packages {
            //     cmd.args.push("--filter".into());
            //     cmd.args.push(package_name);
            // }

            cmd
        }
        JavaScriptPackageManager::Npm => {
            let mut cmd = ExecCommandInput::new(
                "npm",
                if env.ci
                    && (input.root.join("package-lock.json").exists()
                        || input.root.join("npm-shrinkwrap.json").exists())
                {
                    ["ci"]
                } else {
                    ["install"]
                },
            );

            if input.production {
                cmd.args.push("--production".into());
            }

            for package_name in input.packages {
                cmd.args.push("--workspace".into());
                cmd.args.push(package_name);
            }

            cmd
        }
        JavaScriptPackageManager::Pnpm => {
            let mut cmd = ExecCommandInput::new("pnpm", ["install"]);

            if input.production {
                cmd.args.push("--prod".into());
            }

            for package_name in input.packages {
                cmd.args.push(if input.production {
                    "--filter-prod".into()
                } else {
                    "--filter".into()
                });

                // https://pnpm.io/filtering#--filter-package_name-1
                cmd.args.push(format!("{package_name}..."));
            }

            cmd
        }
        JavaScriptPackageManager::Yarn => {
            let mut cmd = ExecCommandInput::new("yarn", ["install"]);

            if !input.packages.is_empty() && package_manager_config.version_satisfies(">=2.0.0") {
                cmd = ExecCommandInput::new("yarn", ["workspaces", "focus"]);
                cmd.args.extend(input.packages);

                inherit_install_args = false;
            };

            if input.production {
                cmd.args.push("--production".into());
            }

            cmd
        }
    };

    if inherit_install_args {
        command
            .args
            .extend(package_manager_config.install_args.clone());
    }

    command.cwd = Some(input.root.clone());

    output.install_command = Some(command.into());

    // Dedupe
    if config.dedupe_on_lockfile_change {
        match package_manager {
            JavaScriptPackageManager::Bun | JavaScriptPackageManager::Deno => {
                // N/A
            }
            JavaScriptPackageManager::Npm => {
                output.dedupe_command = Some(
                    ExecCommandInput::new("npm", ["dedupe"])
                        .cwd(input.root)
                        .into(),
                );
            }
            JavaScriptPackageManager::Pnpm => {
                output.dedupe_command =
                    Some(if package_manager_config.version_satisfies(">=7.26.0") {
                        ExecCommandInput::new("pnpm", ["dedupe"])
                            .cwd(input.root)
                            .into()
                    } else {
                        ExecCommandInput::new(
                            "npx",
                            [
                                "--quiet",
                                "--package",
                                "pnpm-deduplicate",
                                "--",
                                "pnpm-deduplicate",
                            ],
                        )
                        .cwd(input.root)
                        .into()
                    });
            }
            JavaScriptPackageManager::Yarn => {
                output.dedupe_command =
                    Some(if package_manager_config.version_satisfies(">=2.0.0") {
                        ExecCommandInput::new("yarn", ["dedupe"])
                            .cwd(input.root)
                            .into()
                    } else {
                        ExecCommandInput::new(
                            "npx",
                            [
                                "--quiet",
                                "--package",
                                "yarn-deduplicate",
                                "--",
                                "yarn-deduplicate",
                                "yarn.lock",
                            ],
                        )
                        .cwd(input.root)
                        .into()
                    });
            }
        };
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();

    match input.path.file_name().and_then(|name| name.to_str()) {
        Some("bun.lock") => parse_bun_lock(&input.path, &mut output)?,
        Some("bun.lockb") => parse_bun_lockb(&input.path, &mut output)?,
        Some("package-lock.json" | "npm-shrinkwrap.json") => {
            parse_package_lock_json(&input.path, &mut output)?
        }
        Some("pnpm-lock.yaml") => parse_pnpm_lock_yaml(&input.path, &mut output)?,
        Some("yarn.lock") => parse_yarn_lock(&input.path, &mut output)?,
        _ => {}
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let manifest = PackageJson::load(input.path)?;
    let mut output = ParseManifestOutput::default();

    let extract_deps = |in_deps: &BTreeMap<String, VersionProtocol>,
                        out_deps: &mut BTreeMap<String, ManifestDependency>|
     -> AnyResult<()> {
        for (name, version) in in_deps {
            let dep = match version {
                VersionProtocol::Alias(_) | VersionProtocol::Catalog(_) => {
                    continue;
                }
                VersionProtocol::File(path)
                | VersionProtocol::Link(path)
                | VersionProtocol::Portal(path) => ManifestDependency::path(path.to_owned()),
                VersionProtocol::Git { url, .. } => ManifestDependency::url(url.to_owned()),
                VersionProtocol::GitHub {
                    reference,
                    owner,
                    repo,
                } => ManifestDependency::url(format!(
                    "https://github.com/{owner}/{repo}.git#{}",
                    reference.as_deref().unwrap_or("master")
                )),
                VersionProtocol::Range(version_reqs) => {
                    ManifestDependency::Version(UnresolvedVersionSpec::parse(
                        version_reqs
                            .iter()
                            .map(|req| req.to_string())
                            .collect::<Vec<_>>()
                            .join(" || "),
                    )?)
                }
                VersionProtocol::Requirement(version_req) => ManifestDependency::Version(
                    UnresolvedVersionSpec::parse(version_req.to_string())?,
                ),
                VersionProtocol::Tag(tag) => {
                    ManifestDependency::Version(UnresolvedVersionSpec::parse(tag)?)
                }
                VersionProtocol::Url(url) => ManifestDependency::url(url.to_owned()),
                VersionProtocol::Version(version) => {
                    ManifestDependency::Version(UnresolvedVersionSpec::parse(version.to_string())?)
                }
                VersionProtocol::Workspace(ws) => match ws {
                    WorkspaceProtocol::File(path) => ManifestDependency::path(path.to_owned()),
                    WorkspaceProtocol::Version(version) => ManifestDependency::Version(
                        UnresolvedVersionSpec::parse(version.to_string())?,
                    ),
                    _ => {
                        continue;
                    }
                },
            };

            out_deps.insert(name.to_owned(), dep);
        }

        Ok(())
    };

    if let Some(deps) = &manifest.dependencies {
        extract_deps(deps, &mut output.dependencies)?;
    }

    if let Some(deps) = &manifest.dev_dependencies {
        extract_deps(deps, &mut output.dev_dependencies)?;
    }

    if let Some(deps) = &manifest.peer_dependencies {
        extract_deps(deps, &mut output.peer_dependencies)?;
    }

    if let Some(deps) = &manifest.optional_dependencies {
        extract_deps(deps, &mut output.build_dependencies)?;
    }

    if let Some(version) = &manifest.version {
        output.version = Some(version.to_owned());
    }

    output.publishable = manifest.version.is_some()
        && (manifest.main.is_some() || manifest.module.is_some() || manifest.exports.is_some())
        && manifest.workspaces.is_none();

    Ok(Json(output))
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config)?;
    let mut output = SetupEnvironmentOutput::default();

    // Sync `packageManager` field
    if config.sync_package_manager_field
        && let Some(package_manager) = config.package_manager
        && package_manager.is_for_node()
    {
        let package_manager_config =
            load_toolchain_config::<SharedPackageManagerConfig>(package_manager.to_string())?;
        let package_path = input.root.join("package.json");

        if package_path.exists()
            && let Some(version) = package_manager_config.version
            && matches!(version, UnresolvedVersionSpec::Semantic(_))
        {
            let (op, file) = Operation::track("sync-package-manager", || {
                let mut package = PackageJson::load(package_path)?;
                package.set_package_manager(format!("{package_manager}@{version}"))?;
                package.save()
            })?;

            output.operations.push(op);

            if let Some(file) = file.and_then(|file| file.virtual_path()) {
                output.changed_files.push(file);
            }
        }
    }

    Ok(Json(output))
}
