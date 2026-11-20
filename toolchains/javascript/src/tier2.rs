use crate::config::*;
use crate::infer_tasks::TasksInferrer;
use crate::lockfiles::*;
use crate::nub::{NUB_LOCK_NAMES, detect_nub_identity};
use crate::package_json::PackageJson;
use crate::yarn::detect_yarn_major;
use extism_pdk::*;
use moon_common::path::paths_are_equal;
use moon_config::DependencyScope;
use moon_pdk::{
    VirtualPathExt, get_host_environment, load_project_toolchain_config, load_toolchain_config,
    locate_root_many, locate_root_many_with_check, parse_toolchain_config_schema, plugin_err,
};
use moon_pdk_api::*;
use nodejs_package_json::{
    MatchesVersion, Op, Version as PackageVersion, VersionProtocol, WorkspaceProtocol,
};
use rustc_hash::FxHashMap;
use starbase_utils::yaml;
use std::collections::BTreeMap;
use std::path::PathBuf;

struct GraphPackage {
    id: Id,
    root: VirtualPath,
    name: Option<String>,
    // Deno projects may only have a `deno.json`
    manifest: Option<PackageJson>,
    deno: Option<(VirtualPath, DenoJson)>,
}

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendProjectGraphOutput::default();

    // First pass, gather all packages and their manifests
    let mut packages = vec![];
    let mut packages_by_name = FxHashMap::default();
    let is_deno = config.package_manager == Some(JavaScriptPackageManager::Deno);

    for (id, source) in input.project_sources {
        let project_root = input.context.get_project_root_from_source(&source);
        let package_path = project_root.join("package.json");

        let manifest = if package_path.exists() {
            let mut manifest = PackageJson::load(package_path)?;

            // Remove fields we don't need to avoid eating a ton of memory
            manifest.browser = None;
            manifest.imports = None;
            manifest.exports = None;
            manifest.resolutions = None;
            manifest.dev_engines = None;
            manifest.engines = None;
            manifest.workspaces = None;
            manifest.catalog = None;
            manifest.catalogs = None;
            manifest.other_fields.clear();

            Some(manifest)
        } else {
            None
        };

        // Deno workspaces usually only have a `deno.json`
        let deno = if is_deno {
            DenoJson::find(&project_root)?
        } else {
            None
        };

        if manifest.is_none() && deno.is_none() {
            continue;
        }

        let name = deno
            .as_ref()
            .and_then(|(_, deno)| deno.name.clone())
            .or_else(|| manifest.as_ref().and_then(|manifest| manifest.name.clone()));

        // Projects can share a package name, like examples or fixtures,
        // so the first one is the dependency target, but all are kept
        if let Some(name) = &name {
            packages_by_name
                .entry(name.to_owned())
                .or_insert(packages.len());
        }

        packages.push(GraphPackage {
            id,
            root: project_root,
            name,
            manifest,
            deno,
        });
    }

    // npm, Yarn and Bun link a workspace package when its version satisfies
    // a range, while pnpm only does with `link-workspace-packages`
    let links_ranges = matches!(
        config.package_manager,
        Some(
            JavaScriptPackageManager::Bun
                | JavaScriptPackageManager::Npm
                | JavaScriptPackageManager::Yarn
        )
    );

    let get_package = |name: &str| {
        packages_by_name
            .get(name)
            .and_then(|index| packages.get(*index))
    };

    // Second pass, extract packages and their relationships
    for package in &packages {
        let project_root = &package.root;
        let mut project_output = ExtendProjectOutput {
            alias: package.name.clone(),
            ..Default::default()
        };

        let mut extract_implicit_deps =
            |package_deps: &Option<BTreeMap<String, VersionProtocol>>,
             scope: DependencyScope|
             -> AnyResult<()> {
                let Some(deps) = package_deps else {
                    return Ok(());
                };

                for (dep_name, dep_version) in deps {
                    let Some(dep) = get_package(dep_name) else {
                        continue;
                    };

                    let satisfies = |matches: &dyn Fn(&PackageVersion) -> bool| {
                        links_ranges
                            && dep
                                .manifest
                                .as_ref()
                                .and_then(|manifest| manifest.version.as_ref())
                                .is_some_and(matches)
                    };

                    // Only inherit if the dependency is in the local workspace
                    let is_local = match dep_version {
                        VersionProtocol::File(rel_path)
                        | VersionProtocol::Link(rel_path)
                        | VersionProtocol::Path(rel_path)
                        | VersionProtocol::Portal(rel_path) => {
                            paths_are_equal(&dep.root, project_root.join(rel_path))
                        }
                        // *
                        VersionProtocol::Requirement(req)
                            if req.op == Op::Wildcard && req.major.is_none() =>
                        {
                            true
                        }
                        VersionProtocol::Requirement(req) => satisfies(&|ver| req.matches(ver)),
                        VersionProtocol::Range(range) => satisfies(&|ver| range.matches(ver)),
                        VersionProtocol::Version(version) => satisfies(&|ver| ver == version),
                        VersionProtocol::Workspace(_) => true,
                        _ => false,
                    };

                    if is_local {
                        project_output.dependencies.push(ProjectDependency {
                            id: dep.id.to_owned(),
                            scope,
                            via: Some(format!("package {dep_name}")),
                        });
                    }
                }

                Ok(())
            };

        if let Some(manifest) = &package.manifest {
            extract_implicit_deps(&manifest.dependencies, DependencyScope::Production)?;
            extract_implicit_deps(&manifest.dev_dependencies, DependencyScope::Development)?;
            extract_implicit_deps(&manifest.peer_dependencies, DependencyScope::Peer)?;
            extract_implicit_deps(&manifest.optional_dependencies, DependencyScope::Build)?;

            output.input_files.push(manifest.path.clone());
        }

        // Deno resolves a `jsr:` or `npm:` import of a workspace member, with
        // any version, to the member. Imports only in source files aren't
        // linked, as finding them needs `deno info` for every project
        if let Some((path, deno)) = &package.deno {
            for (alias, specifier) in &deno.imports {
                if let Some(dep_name) = get_import_package_name(specifier)
                    && let Some(dep) = get_package(&dep_name)
                    && dep.id != package.id
                    && !project_output
                        .dependencies
                        .iter()
                        .any(|existing| existing.id == dep.id)
                {
                    project_output.dependencies.push(ProjectDependency {
                        id: dep.id.to_owned(),
                        scope: DependencyScope::Production,
                        via: Some(format!("import {alias}")),
                    });
                }
            }

            output.input_files.push(path.clone());
        }

        if config.infer_tasks_from_scripts
            && let Some(package_manager) = config.package_manager
        {
            let scripts = package
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.scripts.as_ref());

            if package_manager == JavaScriptPackageManager::Deno {
                let deno_tasks = package.deno.as_ref().map(|(_, deno)| &deno.tasks);

                // `deno task` also runs `package.json` scripts, unless
                // `deno.json` has a task with the same name
                if let Some(scripts) = scripts {
                    project_output.tasks.extend(
                        TasksInferrer::new(&config).infer_from_package_scripts(
                            scripts
                                .iter()
                                .filter(|(name, _)| {
                                    !deno_tasks.is_some_and(|tasks| tasks.contains_key(*name))
                                })
                                .collect(),
                        )?,
                    );
                }

                if let Some(tasks) = deno_tasks {
                    project_output
                        .tasks
                        .extend(TasksInferrer::new(&config).infer_from_deno_tasks(tasks)?);
                }
            } else if let Some(scripts) = scripts {
                project_output.tasks.extend(
                    TasksInferrer::new(&config)
                        .infer_from_package_scripts(BTreeMap::from_iter(scripts))?,
                );
            }
        }

        output
            .extended_projects
            .insert(package.id.to_owned(), project_output);
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
        if let Some(bin_dir) = current_dir
            .join("node_modules")
            .join(".bin")
            .to_real_path()?
        {
            paths.push(bin_dir.to_path_buf());
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
        .to_real_path()?
    {
        paths.push(bin_dir.to_path_buf());
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
    let mut output = DefineRequirementsOutput {
        for_setup_toolchain: true,
        ..Default::default()
    };

    if let Some(package_manager) = config.package_manager {
        if !package_manager.is_standalone() {
            output.requires.push("node".into());
        }

        output
            .requires
            .push(package_manager.get_toolchain_id().to_string());
    }

    Ok(Json(output))
}

fn get_var_key(prefix: &str, root: &VirtualPath) -> String {
    format!("{prefix}:{}", root.to_string().trim_end_matches('/'))
}

fn load_catalogs(root: &VirtualPath) -> AnyResult<CatalogsMap> {
    if let Some(data) = var::get::<String>(get_var_key("workspace-catalogs", root))? {
        return Ok(json::from_str(&data)?);
    }

    Ok(FxHashMap::default())
}

pub(crate) fn extract_workspace_members_and_catalogs(
    package_manager: JavaScriptPackageManager,
    root: &VirtualPath,
) -> AnyResult<Option<Vec<String>>> {
    let members_cache_key = get_var_key("workspace-members", root);
    let catalogs_cache_key = get_var_key("workspace-catalogs", root);
    let mut members = None;
    let mut catalogs = None;

    // Reduce the amount of file system operations for every package
    // within the workspace by caching the found members for this directory
    if let Some(cache) = var::get::<String>(&members_cache_key)? {
        let members: Vec<String> = json::from_str(&cache)?;

        return Ok(Some(members));
    }

    // Nub only reads the config files of the package manager that owns the repo,
    // and warns that `pnpm-workspace.yaml` is not read in repos it owns
    let config_manager = if package_manager == JavaScriptPackageManager::Nub {
        detect_nub_identity(root)?
    } else {
        package_manager
    };

    // Package manager specific files
    match config_manager {
        // Deno uses the members of both files, while `package.json`'s
        // catalogs replace `deno.json`'s when both define them
        JavaScriptPackageManager::Deno => {
            let deno = DenoJson::load_from(root)?;
            let package_json = PackageJson::load(root.join("package.json"))?;

            catalogs = package_json
                .extract_catalogs()
                .or_else(|| deno.extract_catalogs());

            let mut list = deno
                .workspace
                .map(|ws| ws.get_members().to_vec())
                .unwrap_or_default();

            for member in package_json.extract_members().unwrap_or_default() {
                if !list.contains(&member) {
                    list.push(member);
                }
            }

            members = (!list.is_empty()).then_some(list);
        }
        JavaScriptPackageManager::Pnpm => {
            let workspace_file = root.join("pnpm-workspace.yaml");

            if workspace_file.exists() {
                let workspace: PnpmWorkspace = yaml::read_file(workspace_file)?;

                catalogs = workspace.extract_catalogs();
                members = workspace.packages;
            }
        }
        JavaScriptPackageManager::Yarn => {
            let yarnrc_file = root.join(".yarnrc.yml");

            if yarnrc_file.exists() {
                let yarnrc: YarnRc = yaml::read_file(yarnrc_file)?;

                catalogs = yarnrc.extract_catalogs();
            }
        }
        _ => {}
    };

    // Otherwise `package.json` itself
    if package_manager != JavaScriptPackageManager::Deno
        && (catalogs.is_none() || members.is_none())
    {
        let package_json = PackageJson::load(root.join("package.json"))?;

        if catalogs.is_none() {
            catalogs = package_json.extract_catalogs();
        }

        if members.is_none() {
            members = package_json.extract_members();
        }
    }

    // Cache the result! The cache only saves work, so a full var store
    // (1 MiB in Extism) shouldn't fail the call
    if let Some(catalogs) = &catalogs
        && let Err(error) = var::set(catalogs_cache_key, json::to_string(catalogs)?)
    {
        debug!("Failed to cache the workspace catalogs: {error}");
    }

    if let Some(members) = &members
        && let Err(error) = var::set(members_cache_key, json::to_string(members)?)
    {
        debug!("Failed to cache the workspace members: {error}");
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

    let package_manager_config: SharedPackageManagerConfig =
        load_toolchain_config(package_manager.get_toolchain_id())?;

    let manifest_names = match package_manager {
        JavaScriptPackageManager::Deno => vec!["deno.json", "deno.jsonc", "package.json"],
        _ => vec!["package.json"],
    };

    let workspace_manifest_names = match package_manager {
        JavaScriptPackageManager::Deno => vec!["deno.json", "deno.jsonc", "package.json"],
        JavaScriptPackageManager::Nub | JavaScriptPackageManager::Pnpm => {
            vec!["pnpm-workspace.yaml", "package.json"]
        }
        _ => vec!["package.json"],
    };

    let lock_names = match package_manager {
        JavaScriptPackageManager::Bun => vec!["bun.lock", "bun.lockb"],
        JavaScriptPackageManager::Deno => {
            // Deno v2.9 can seed deno.lock from existing Node package manager
            // lockfiles, so treat them as valid dependency roots for migration
            if package_manager_config.version_satisfies(">=2.9.0") {
                vec![
                    "deno.lock",
                    "package-lock.json",
                    "npm-shrinkwrap.json",
                    "pnpm-lock.yaml",
                    "yarn.lock",
                    "bun.lock",
                ]
            } else {
                vec!["deno.lock"]
            }
        }
        JavaScriptPackageManager::Npm => vec!["package-lock.json", "npm-shrinkwrap.json"],
        JavaScriptPackageManager::Nub => NUB_LOCK_NAMES.to_vec(),
        JavaScriptPackageManager::Pnpm => vec!["pnpm-lock.yaml"],
        JavaScriptPackageManager::Yarn => vec!["yarn.lock"],
    };

    // First attempt: find lock files
    if let Some(root) = locate_root_many(&input.starting_dir, &lock_names) {
        output.members = extract_workspace_members_and_catalogs(package_manager, &root)?;
        output.root = Some(root);
    }

    // Second attempt: find workspace-compatible manifest files
    if output.root.is_none() {
        locate_root_many_with_check(&input.starting_dir, &workspace_manifest_names, |root| {
            let mut found = false;

            if let Some(members) = extract_workspace_members_and_catalogs(package_manager, root)? {
                output.root = Some(root.to_owned());
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
        extract_workspace_members_and_catalogs(package_manager, &root)?;

        output.root = Some(root);
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
    let mut inherit_install_args = true;

    let package_manager_config: SharedPackageManagerConfig = match &input.project {
        Some(project) => {
            load_project_toolchain_config(&project.id, package_manager.get_toolchain_id())?
        }
        None => load_toolchain_config(package_manager.get_toolchain_id())?,
    };

    let yarn_major = if package_manager == JavaScriptPackageManager::Yarn {
        detect_yarn_major(package_manager_config.version.as_ref(), &input.root)?
    } else {
        0
    };

    // Install
    let mut command = match package_manager {
        JavaScriptPackageManager::Bun => {
            let mut cmd = ExecCommandInput::new("bun", ["install"]);

            // Unlike other package managers, Bun doesn't freeze the lockfile
            // in CI. Every Bun has `--frozen-lockfile` (`bun ci` is an alias),
            // which also works with `bun.lockb`, and `--production` implies it
            if input.production {
                cmd.args.push("--production".into());
            } else if env.ci
                && ["bun.lock", "bun.lockb"]
                    .iter()
                    .any(|name| input.root.join(name).exists())
            {
                cmd.args.push("--frozen-lockfile".into());
            }

            for package_name in input.packages {
                cmd.args.push("--filter".into());

                // Dependency relation patterns were introduced in v1.4,
                // and are required to also install workspace dependencies
                // https://bun.com/docs/pm/filter
                cmd.args
                    .push(if package_manager_config.version_satisfies(">=1.4.0") {
                        format!("{package_name}...")
                    } else {
                        package_name
                    });
            }

            cmd
        }
        JavaScriptPackageManager::Deno => {
            let mut cmd = ExecCommandInput::new("deno", ["install"]);

            // Deno doesn't freeze the lockfile in CI. `deno ci` (v2.8+) only adds
            // removing `node_modules`, and rejects other args, like `--allow-scripts`
            if env.ci && input.root.join("deno.lock").exists() {
                cmd.args.push("--frozen".into());
            }

            // `--prod` was introduced in v2.8 to skip dev dependencies
            // and type packages for production deployments.
            if input.production && package_manager_config.version_satisfies(">=2.8.0") {
                cmd.args.push("--prod".into());
            }

            // Deno has no filter for focused installs, like `--filter`
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

            // `--production` is deprecated, and warns since npm 12
            if input.production {
                cmd.args.push("--omit=dev".into());
            }

            // `--workspace` only accepts members, so a project at the root fails it
            let root_name = if input.packages.is_empty() {
                None
            } else {
                PackageJson::load(input.root.join("package.json"))?
                    .data
                    .name
            };
            let mut has_workspace = false;

            for package_name in input.packages {
                if root_name.as_ref() != Some(&package_name) {
                    cmd.args.push("--workspace".into());
                    cmd.args.push(package_name);
                    has_workspace = true;
                }
            }

            // Otherwise the root package's own dependencies aren't installed
            if has_workspace {
                cmd.args.push("--include-workspace-root".into());
            }

            cmd
        }
        JavaScriptPackageManager::Nub => {
            let mut cmd = ExecCommandInput::new("nub", ["install"]);

            // nub only freezes the lockfile when `CI` is set, and `nub ci` rejects
            // most install args, like `--prefer-offline`. It also rejects
            // `--frozen-lockfile` with `--no-frozen-lockfile`.
            if env.ci
                && NUB_LOCK_NAMES
                    .iter()
                    .any(|name| input.root.join(name).exists())
                && !package_manager_config
                    .install_args
                    .iter()
                    .any(|arg| arg.ends_with("frozen-lockfile"))
            {
                cmd.args.push("--frozen-lockfile".into());
            }

            if input.production {
                cmd.args.push("--prod".into());

                // Outside CI, nub links packages from a store shared by the machine,
                // which a Docker image's later stages don't have
                cmd.env.insert(
                    "npm_config_enable_global_virtual_store".into(),
                    "false".into(),
                );
            }

            for package_name in input.packages {
                cmd.args.push(if input.production {
                    "--filter-prod".into()
                } else {
                    "--filter".into()
                });

                // https://nubjs.com/docs/install#nub-install
                cmd.args.push(format!("{package_name}..."));
            }

            cmd
        }
        JavaScriptPackageManager::Pnpm => {
            let mut cmd = ExecCommandInput::new("pnpm", ["install"]);

            if input.production {
                cmd.args.push("--prod".into());

                // When dev dependencies are installed, pnpm asks before purging
                // `node_modules`, which aborts without a TTY, like in Docker
                cmd.args.push("--config.confirmModulesPurge=false".into());
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
        // Yarn 2+ removed `install --production`, and focuses workspaces instead
        JavaScriptPackageManager::Yarn
            if yarn_major >= 2 && (input.production || !input.packages.is_empty()) =>
        {
            if yarn_major < 4
                && !input
                    .root
                    .join(".yarn/plugins/@yarnpkg/plugin-workspace-tools.cjs")
                    .exists()
            {
                return Err(plugin_err!(
                    "Yarn {yarn_major} needs the workspace-tools plugin for focused and production installs. Run <shell>yarn plugin import workspace-tools</shell>, and commit <file>.yarn/plugins</file> and <file>.yarnrc.yml</file>."
                ));
            }

            let mut cmd = ExecCommandInput::new("yarn", ["workspaces", "focus"]);

            if input.packages.is_empty() {
                cmd.args.push("--all".into());
            } else {
                cmd.args.extend(input.packages);
            }

            if input.production {
                cmd.args.push("--production".into());
            }

            inherit_install_args = false;

            cmd
        }
        JavaScriptPackageManager::Yarn => {
            let mut cmd = ExecCommandInput::new("yarn", ["install"]);

            if input.production {
                cmd.args.push("--production".into());
            }

            // Yarn 2 to 4 freeze the lockfile in CI themselves (`enableImmutableInstalls`)
            let frozen_arg = match yarn_major {
                1 => Some("--frozen-lockfile"),
                2..=4 => None,
                _ => Some("--immutable"),
            };

            if let Some(arg) = frozen_arg
                && env.ci
                && input.root.join("yarn.lock").exists()
                && !package_manager_config.install_args.iter().any(|arg| {
                    matches!(
                        arg.as_str(),
                        "--frozen-lockfile" | "--pure-lockfile" | "--immutable" | "--no-immutable"
                    )
                })
            {
                cmd.args.push(arg.into());
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
            JavaScriptPackageManager::Bun => {
                // `bun dedupe` was introduced in v1.4, but v1.4.0's can't read `bun.lockb`
                if package_manager_config.version_satisfies(">=1.4.1") {
                    output.dedupe_command = Some(
                        ExecCommandInput::new("bun", ["dedupe"])
                            .cwd(input.root)
                            .into(),
                    );
                }
            }
            JavaScriptPackageManager::Deno => {
                // N/A
            }
            JavaScriptPackageManager::Npm => {
                let mut cmd = ExecCommandInput::new("npm", ["dedupe"]);

                // Dedupe re-resolves the tree, so it needs flags like `--legacy-peer-deps`
                cmd.args.extend(package_manager_config.install_args.clone());
                cmd.cwd = Some(input.root);

                output.dedupe_command = Some(cmd.into());
            }
            JavaScriptPackageManager::Nub => {
                // nub won't write `yarn.lock`, so `nub dedupe` fails there
                if detect_nub_identity(&input.root)? != JavaScriptPackageManager::Yarn {
                    output.dedupe_command = Some(
                        ExecCommandInput::new("nub", ["dedupe"])
                            .cwd(input.root)
                            .into(),
                    );
                }
            }
            JavaScriptPackageManager::Pnpm => {
                output.dedupe_command =
                    Some(if package_manager_config.version_satisfies("<7.26.0") {
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
                    } else {
                        ExecCommandInput::new("pnpm", ["dedupe"])
                            .cwd(input.root)
                            .into()
                    });
            }
            JavaScriptPackageManager::Yarn => {
                // Yarn 1 has no `yarn dedupe`, and fails it
                output.dedupe_command = Some(if yarn_major == 1 {
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
                } else {
                    ExecCommandInput::new("yarn", ["dedupe"])
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
    let file_name = input
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    // moon parses every lockfile that exists and merges them, so a leftover
    // from another package manager would override the real one
    let config: LockConfig = load_toolchain_config("javascript")?;

    if let Some(package_manager) = config.package_manager {
        let own_names = package_manager.get_lock_file_names();

        if !own_names.contains(&file_name)
            && own_names.iter().any(|name| input.root.join(name).exists())
        {
            debug!("Skipping {file_name}, as {package_manager} has its own lockfile");

            return Ok(Json(output));
        }
    }

    let result = match Some(file_name) {
        Some("bun.lock") => parse_bun_lock(&input.path, &mut output),
        Some("bun.lockb") => parse_bun_lockb(&input.path, &mut output),
        Some("deno.lock") => parse_deno_lock(&input.path, &mut output),
        Some("package-lock.json" | "npm-shrinkwrap.json") => {
            parse_package_lock_json(&input.path, &mut output)
        }
        // `nub.lock` uses the pnpm lockfile format
        Some("nub.lock") | Some("pnpm-lock.yaml") => parse_pnpm_lock_yaml(&input.path, &mut output),
        Some("yarn.lock") => parse_yarn_lock(&input.path, &mut output),
        _ => Ok(()),
    };

    // ADR-0008.8
    if let Err(error) = result {
        let path = match input.path.to_real_path() {
            Ok(Some(path)) => path.to_path_buf().display().to_string(),
            _ => input.path.to_string(),
        };

        return Err(anyhow!("Failed to parse <file>{path}</file>: {error}").into());
    }

    Ok(Json(output))
}

// ADR-0008.5: Skip a dependency whose version can't be parsed
fn parse_manifest_version(name: &str, version: impl AsRef<str>) -> Option<ManifestDependency> {
    let version = version.as_ref();

    match UnresolvedVersionSpec::parse(version) {
        Ok(spec) => Some(ManifestDependency::Version(spec)),
        Err(error) => {
            debug!("Skipping the dependency {name} {version} in package.json: {error}");

            None
        }
    }
}

fn create_manifest_dependency(
    name: &str,
    version: &VersionProtocol,
    catalogs: &CatalogsMap,
) -> Option<ManifestDependency> {
    let dep = match version {
        // `npm:string-width@^4.2.0`, hashed by its range under the alias name
        VersionProtocol::Alias(alias) => {
            let package = alias
                .split_once(':')
                .map(|(_, package)| package)
                .unwrap_or(alias);

            return parse_manifest_version(
                name,
                match package.rfind('@') {
                    Some(index) if index > 0 => &package[index + 1..],
                    _ => "*",
                },
            );
        }
        VersionProtocol::Exec(_)
        | VersionProtocol::Patch(_)
        | VersionProtocol::Unknown(_)
        | VersionProtocol::CustomProtocol(_, _) => {
            return None;
        }
        VersionProtocol::Catalog(key) => {
            // `catalog:default` is the default catalog, like `catalog:`
            let catalog = match key {
                Some(key) if key != "default" => catalogs.get(key),
                _ => catalogs.get("__default__"),
            };

            if let Some(catalog) = catalog
                && let Some(version) = catalog.get(name)
            {
                return create_manifest_dependency(name, version, catalogs);
            }

            return None;
        }
        VersionProtocol::File(path)
        | VersionProtocol::Link(path)
        | VersionProtocol::Path(path)
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
        VersionProtocol::Range(range) => {
            return parse_manifest_version(name, range.to_string());
        }
        VersionProtocol::Requirement(version_req) => {
            return parse_manifest_version(name, version_req.to_string());
        }
        // A dist tag like `latest` can't match a locked version, so use `*`,
        // which matches the first locked entry
        VersionProtocol::Tag(_) => {
            return parse_manifest_version(name, "*");
        }
        VersionProtocol::Url(url) => ManifestDependency::url(url.to_owned()),
        VersionProtocol::Version(version) => {
            return parse_manifest_version(name, version.to_string());
        }
        VersionProtocol::Workspace(ws) => match ws {
            WorkspaceProtocol::File(path) => ManifestDependency::path(path.to_owned()),
            WorkspaceProtocol::Range { range, .. } => {
                return parse_manifest_version(name, range.to_string());
            }
            WorkspaceProtocol::Requirement { req, .. } => {
                return parse_manifest_version(name, req.to_string());
            }
            WorkspaceProtocol::Version { version, .. } => {
                return parse_manifest_version(name, version.to_string());
            }
            _ => {
                return None;
            }
        },
    };

    Some(dep)
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let manifest = PackageJson::load(input.path)?;
    let catalogs = load_catalogs(&input.root)?;
    let mut output = ParseManifestOutput::default();

    let extract_deps =
        |in_deps: &BTreeMap<String, VersionProtocol>,
         out_deps: &mut BTreeMap<String, ManifestDependency>| {
            for (name, version) in in_deps {
                if let Some(dep) = create_manifest_dependency(name, version, &catalogs) {
                    out_deps.insert(name.to_owned(), dep);
                }
            }
        };

    if let Some(deps) = &manifest.dependencies {
        extract_deps(deps, &mut output.dependencies);
    }

    if let Some(deps) = &manifest.dev_dependencies {
        extract_deps(deps, &mut output.dev_dependencies);
    }

    if let Some(deps) = &manifest.peer_dependencies {
        extract_deps(deps, &mut output.peer_dependencies);
    }

    if let Some(deps) = &manifest.optional_dependencies {
        extract_deps(deps, &mut output.build_dependencies);
    }

    // ADR-0008.5
    if let Some(version) = &manifest.version {
        match Version::parse(version.to_string()) {
            Ok(version) => output.version = Some(version),
            Err(error) => debug!("Skipping the version {version} in package.json: {error}"),
        };
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
        let package_manager_config: SharedPackageManagerConfig = match &input.project {
            Some(project) => {
                load_project_toolchain_config(&project.id, package_manager.get_toolchain_id())?
            }
            None => load_toolchain_config(package_manager.get_toolchain_id())?,
        };

        let package_path = input.root.join("package.json");

        // Pinning Nub in a repo that another package manager owns would switch nub
        // to its own format, which ignores that manager's config files
        if package_path.exists()
            && let Some(version) = package_manager_config.version
            && matches!(version, UnresolvedVersionSpec::Version(_))
            && (package_manager != JavaScriptPackageManager::Nub
                || detect_nub_identity(&input.root)? == JavaScriptPackageManager::Nub)
        {
            let (op, file) = Operation::track("sync-package-manager", || {
                let mut package = PackageJson::load(package_path)?;

                // pnpm 11+ prefers `devEngines.packageManager`, and warns when `packageManager`
                // differs, so only update `packageManager` beside it. Yarn Switch only reads
                // `packageManager`.
                if package_manager != JavaScriptPackageManager::Yarn
                    && package.set_dev_engine_package_manager(
                        &package_manager.to_string(),
                        &version.to_string(),
                    )
                {
                    if package.data.package_manager.is_some() {
                        package.set_package_manager(format!("{package_manager}@{version}"))?;
                    }
                } else {
                    package.set_package_manager(format!("{package_manager}@{version}"))?;
                }

                package.save()
            })?;

            output.operations.push(op);

            if let Some(file) = file {
                output.changed_files.push(file);
            }
        }
    }

    Ok(Json(output))
}
