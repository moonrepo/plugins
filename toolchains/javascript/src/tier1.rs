use crate::config::*;
use crate::package_json::PackageJson;
use crate::tier2::extract_workspace_members_and_catalogs;
use extism_pdk::*;
use moon_common::path::{is_root_level_source, to_relative_virtual_string};
use moon_config::{DependencyScope, LanguageType};
use moon_pdk::{
    HostLogInput, host_log, is_project_toolchain_enabled, map_miette_error,
    parse_toolchain_config_schema, plugin_err,
};
use moon_pdk_api::*;
use nodejs_package_json::{Version, VersionProtocol, WorkspaceProtocol};
use schematic::SchemaBuilder;
use starbase_utils::{fs, glob, json::JsonValue};
use std::str::FromStr;
use toolchain_common::{enable_tracing, resolve_source_path};

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: "JavaScript".into(),
        description: Some(
            "Installs dependencies and provides sync operations that keep <file>package.json</file>'s in a healthy state."
                .into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        language: Some(LanguageType::JavaScript),
        // For project detection
        config_file_globs: vec![
            // bun
            "bunfig.toml".into(),
            // deno
            "deno.json".into(),
            "deno.jsonc".into(),
            // npm
            ".npmrc".into(),
            // nub
            "nub.jsonc".into(),
            // pnpm
            "pnpm-workspace.yaml".into(),
            ".pnpmfile.*".into(),
            // yarn
            ".yarn/**/*".into(),
            ".yarnrc".into(),
            ".yarnrc.*".into(),
        ],
        manifest_file_names: vec!["package.json".into()],
        lock_file_names: vec![
            // bun
            "bun.lock".into(),
            "bun.lockb".into(),
            // deno
            "deno.lock".into(),
            // npm
            "package-lock.json".into(),
            "npm-shrinkwrap.json".into(),
            // nub
            "nub.lock".into(),
            // pnpm
            "pnpm-lock.yaml".into(),
            // yarn
            "yarn.lock".into(),
        ],
        // For task detection
        exe_names: vec![
            // bun
            "bun".into(),
            "bunx".into(),
            // deno
            "deno".into(),
            // node
            "node".into(),
            "nodejs".into(),
            // npm
            "npm".into(),
            "npx".into(),
            // nub
            "nub".into(),
            "nubx".into(),
            // pnpm
            "pnpm".into(),
            "pnpx".into(),
            "pn".into(),
            "pnx".into(),
            // yarn
            "yarn".into(),
            "yarnpkg".into()
        ],
        vendor_dir_name: Some("node_modules".into()),
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(input): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    let mut output = InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/guides/javascript/node-handbook".into()),
        docs_url: Some("https://moonrepo.dev/docs/config/toolchain#javascript".into()),
        ..Default::default()
    };

    if let Some(package_manager) = detect_package_manager(&input.context.working_dir)? {
        output.default_settings.insert(
            "packageManager".into(),
            JsonValue::String(package_manager.to_string()),
        );
    } else {
        output.prompts.push(SettingPrompt::new(
            "packageManager",
            "Package manager to install <file>package.json</file> dependencies with?",
            PromptType::Select {
                default_index: 2,
                options: vec![
                    JsonValue::String("bun".into()),
                    JsonValue::String("deno".into()),
                    JsonValue::String("npm".into()),
                    JsonValue::String("nub".into()),
                    JsonValue::String("pnpm".into()),
                    JsonValue::String("yarn".into()),
                ],
            },
        ));
    }

    output.prompts.push(SettingPrompt::new(
        "dedupeOnLockfileChange",
        "Automatically dedupe lockfile when changed or after installing dependencies?",
        PromptType::Confirm { default: true },
    ));

    output.prompts.push(SettingPrompt::new(
        "syncProjectWorkspaceDependencies",
        "Sync project-to-project relationships as <file>package.json</file> <property>dependencies</property>?",
        PromptType::Confirm { default: true },
    ));

    output.prompts.push(SettingPrompt::new(
        "inferTasksFromScripts",
        "Infer <file>package.json</file> scripts as moon tasks?",
        PromptType::Confirm { default: false },
    ));

    Ok(Json(output))
}

fn detect_package_manager(root: &VirtualPath) -> AnyResult<Option<JavaScriptPackageManager>> {
    let package = PackageJson::load(root.join("package.json"))?;

    // `pnpm init` (v11+) and nub write `devEngines.packageManager`
    let declared = package
        .package_manager
        .as_ref()
        .map(|pm| pm.split_once('@').map(|parts| parts.0).unwrap_or(pm))
        .into_iter()
        .chain(
            package
                .dev_engines
                .as_ref()
                .and_then(|engines| engines.package_manager.as_ref())
                .and_then(|engines| engines.list().first().map(|engine| engine.name.as_str())),
        );

    for name in declared {
        match JavaScriptPackageManager::from_str(name) {
            Ok(package_manager) => return Ok(Some(package_manager)),
            Err(error) => debug!("Skipping the package manager {name} in package.json: {error}"),
        };
    }

    if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
        return Ok(Some(JavaScriptPackageManager::Bun));
    } else if root.join("deno.lock").exists() {
        return Ok(Some(JavaScriptPackageManager::Deno));
    } else if root.join("package-lock.json").exists() || root.join("npm-shrinkwrap.json").exists() {
        return Ok(Some(JavaScriptPackageManager::Npm));
    } else if root.join("nub.lock").exists() {
        return Ok(Some(JavaScriptPackageManager::Nub));
    } else if root.join("pnpm-lock.yaml").exists() || root.join("pnpm-workspace.yaml").exists() {
        return Ok(Some(JavaScriptPackageManager::Pnpm));
    } else if root.join("yarn.lock").exists() {
        return Ok(Some(JavaScriptPackageManager::Yarn));
    }

    Ok(None)
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<JavaScriptToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn sync_project(Json(input): Json<SyncProjectInput>) -> FnResult<Json<SyncOutput>> {
    let mut output = SyncOutput::default();
    let package_path = input
        .context
        .get_project_root(&input.project)
        .join("package.json");

    // Does not apply to root projects, or projects without a manifest
    if !is_project_toolchain_enabled(&input.project)
        || is_root_level_source(&input.project.source)
        || !package_path.exists()
    {
        output.skipped = true;

        return Ok(Json(output));
    }

    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config.clone())?;
    let mut package = PackageJson::load(package_path)?;
    let dep_packages = load_dependency_packages(&input)?;

    // Enforce single version policy, but allow local workspace packages,
    // which the sync below adds to this same file
    if config.root_package_dependencies_only {
        let has_external_deps = [
            &package.dependencies,
            &package.dev_dependencies,
            &package.peer_dependencies,
            &package.optional_dependencies,
        ]
        .into_iter()
        .flatten()
        .flat_map(|deps| deps.keys())
        .any(|name| !dep_packages.iter().any(|dep| dep.name == *name));

        if has_external_deps {
            return Err(plugin_err!(
                "Dependencies can only be defined in the root <file>package.json</file>, found dependencies in project <id>{}</id>.\nThis is enforced through the <property>javascript.rootPackageDependenciesOnly</property> toolchain setting.",
                input.project.id
            ));
        }
    }

    // Sync workspace dependencies
    if config.sync_project_workspace_dependencies
        && let Some(package_manager) = &config.package_manager
    {
        let (op, _) = Operation::track("sync-project-workspace-dependencies", || {
            sync_project_workspace_dependencies(
                &input,
                &config,
                &mut package,
                package_manager,
                &dep_packages,
            )
        })?;

        output.operations.push(op);

        if let Some(file) = package.save()? {
            output.changed_files.push(file);
        }
    }

    Ok(Json(output))
}

struct DependencyPackage<'input> {
    name: String,
    project: &'input ProjectFragment,
    root: VirtualPath,
    version: Option<Version>,
}

fn load_dependency_packages(input: &SyncProjectInput) -> AnyResult<Vec<DependencyPackage<'_>>> {
    let mut packages = vec![];

    for dep_project in &input.project_dependencies {
        // Do not sync with the root project
        if !is_project_toolchain_enabled(dep_project)
            || is_root_level_source(&dep_project.source)
            || dep_project
                .dependency_scope
                .as_ref()
                .is_some_and(|scope| *scope == DependencyScope::Root)
        {
            continue;
        }

        let root = input.context.get_project_root(dep_project);
        let package_path = root.join("package.json");

        // Only sync if a package exists
        if !package_path.exists() {
            continue;
        }

        let package = PackageJson::load(package_path)?;

        if let Some(name) = &package.name {
            packages.push(DependencyPackage {
                name: name.to_owned(),
                project: dep_project,
                root,
                version: package.version.clone(),
            });
        }
    }

    Ok(packages)
}

/// Whether an existing semver range no longer matches the version. Other
/// protocols, like `workspace:` or paths, are never stale.
fn is_stale_range(current: Option<&VersionProtocol>, version: &Version) -> bool {
    match current {
        Some(VersionProtocol::Requirement(req)) => !req.matches(version),
        Some(VersionProtocol::Range(reqs)) => !reqs.iter().any(|req| req.matches(version)),
        Some(VersionProtocol::Version(current)) => current != version,
        _ => false,
    }
}

fn sync_project_workspace_dependencies(
    input: &SyncProjectInput,
    config: &JavaScriptToolchainConfig,
    package: &mut PackageJson,
    package_manager: &JavaScriptPackageManager,
    dep_packages: &[DependencyPackage],
) -> AnyResult<()> {
    let project_root = input.context.get_project_root(&input.project);
    let version_format = config
        .dependency_version_format
        .get_supported_for(package_manager);
    let version_prefix = version_format.get_prefix();
    let is_version_format = matches!(
        version_format,
        JavaScriptDependencyVersionFormat::Version
            | JavaScriptDependencyVersionFormat::VersionCaret
            | JavaScriptDependencyVersionFormat::VersionTilde
    );

    for dep in dep_packages {
        let Some(scope) = dep.project.dependency_scope.as_ref() else {
            continue;
        };

        let name = &dep.name;
        let version = dep.version.as_ref();

        let protocol = match version_format {
            JavaScriptDependencyVersionFormat::File | JavaScriptDependencyVersionFormat::Link => {
                VersionProtocol::try_from(format!(
                    "{version_prefix}{}",
                    to_relative_virtual_string(&dep.root, &project_root)
                        .map_err(map_miette_error)?
                ))?
            }
            JavaScriptDependencyVersionFormat::Version
            | JavaScriptDependencyVersionFormat::VersionCaret
            | JavaScriptDependencyVersionFormat::VersionTilde => match version {
                Some(version) => VersionProtocol::try_from(format!("{version_prefix}{version}"))?,
                // Only the version formats need the package's version
                None => continue,
            },
            _ => VersionProtocol::from_str(&version_prefix)?,
        };

        // Replace a version range that no longer matches, so package managers
        // don't install an older release from the registry instead
        let replace_if_stale = |current: Option<&VersionProtocol>| {
            is_version_format && version.is_some_and(|version| is_stale_range(current, version))
        };

        match scope {
            DependencyScope::Build | DependencyScope::Root => {
                // Not used
            }
            DependencyScope::Development => {
                let if_missing = !replace_if_stale(
                    package
                        .dev_dependencies
                        .as_ref()
                        .and_then(|deps| deps.get(name)),
                );

                package.add_dev_dependency(name, protocol, if_missing)?;
            }
            DependencyScope::Production => {
                let if_missing = !replace_if_stale(
                    package
                        .dependencies
                        .as_ref()
                        .and_then(|deps| deps.get(name)),
                );

                package.add_dependency(name, protocol, if_missing)?;
            }
            DependencyScope::Peer => {
                let (peer, if_missing) = match version {
                    Some(version) => {
                        let current = package
                            .peer_dependencies
                            .as_ref()
                            .and_then(|deps| deps.get(name));

                        (
                            // `^0.0.0` only matches 0.0.0, so 0.x peers need the minor
                            if version.major == 0 {
                                VersionProtocol::try_from(format!("^{version}"))?
                            } else {
                                VersionProtocol::try_from(format!("^{}.0.0", version.major))?
                            },
                            !is_stale_range(current, version),
                        )
                    }
                    None => (VersionProtocol::from_str("*")?, true),
                };

                package.add_peer_dependency(name, peer, if_missing)?;
            }
        };

        host_log!(
            "Syncing <id>{}</id> as a dependency to <id>{}</id>'s <file>package.json</file>",
            dep.project.id,
            input.project.id,
        );
    }

    Ok(())
}

#[plugin_fn]
pub fn scaffold_docker(
    Json(input): Json<ScaffoldDockerInput>,
) -> FnResult<Json<ScaffoldDockerOutput>> {
    let mut output = ScaffoldDockerOutput::default();

    // Package managers install every workspace member and local directory
    // dependency, but the configs phase only has the manifests of projects
    if input.phase != ScaffoldDockerPhase::Configs {
        return Ok(Json(output));
    }

    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config)?;
    let source = input
        .project
        .as_ref()
        .map(|project| project.source.as_str())
        .filter(|source| *source != ".")
        .unwrap_or_default();

    // Output paths mirror the workspace, starting from the skeleton root
    let mut output_root = input.output_dir.clone();

    for _ in source.split('/').filter(|part| !part.is_empty()) {
        if let Some(parent) = output_root.parent() {
            output_root = parent;
        }
    }

    // Workspace relative directories whose manifest is read
    let mut manifest_dirs = vec![source.to_owned()];

    // Members don't have to be moon projects
    if input.project.is_none()
        && let Some(package_manager) = config.package_manager
        && let Some(members) =
            extract_workspace_members_and_catalogs(package_manager, &input.input_dir)?
    {
        let mut globs = members
            .iter()
            .map(|member| match member.strip_prefix('!') {
                Some(member) => format!("!{}/package.json", member.trim_end_matches('/')),
                None => format!("{}/package.json", member.trim_end_matches('/')),
            })
            .collect::<Vec<_>>();
        globs.push("!**/node_modules/**".into());

        for file in glob::walk_files(&input.input_dir, &globs)? {
            if let Some(dir) = file
                .parent()
                .and_then(|dir| dir.strip_prefix(&input.input_dir).ok())
            {
                manifest_dirs.push(dir.to_string_lossy().into_owned());
            }
        }
    }

    let mut copy_paths = vec![];

    for dir in &manifest_dirs {
        // moon copies the project's or root's own manifest
        if dir != source {
            copy_paths.push(format!("{dir}/package.json"));
        }

        let package =
            PackageJson::load(input.context.workspace_root.join(dir).join("package.json"))?;

        // Local directory and tarball dependencies, like `file:../lib`
        for deps in [
            &package.dependencies,
            &package.dev_dependencies,
            &package.optional_dependencies,
            &package.peer_dependencies,
        ]
        .into_iter()
        .flatten()
        {
            for version in deps.values() {
                let path = match version {
                    VersionProtocol::File(path)
                    | VersionProtocol::Link(path)
                    | VersionProtocol::Portal(path)
                    | VersionProtocol::Workspace(WorkspaceProtocol::File(path)) => path,
                    _ => continue,
                };

                if let Some(target) = resolve_source_path(dir, &path.to_string_lossy()) {
                    let target_path = input.context.workspace_root.join(&target);

                    if target_path.is_dir() {
                        copy_paths.push(format!("{target}/package.json"));
                    } else if target_path.is_file() {
                        copy_paths.push(target);
                    }
                }
            }
        }
    }

    for path in copy_paths {
        let path = path.trim_start_matches('/');
        let file = input.context.workspace_root.join(path);
        let copy = output_root.join(path);

        if file.is_file() && !copy.exists() {
            fs::copy_file(&file, &copy)?;
            output.copied_files.push(copy);
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let mut output = PruneDockerOutput::default();

    if !input.docker_config.delete_vendor_directories {
        return Ok(Json(output));
    }

    // Isolated layouts, like pnpm's and Bun's, leave links to pruned packages
    // in the projects' own `node_modules`
    let mut dirs = vec![input.root.join("node_modules")];

    for project in input.projects.iter().chain(&input.project_dependencies) {
        dirs.push(input.context.get_project_root(project).join("node_modules"));
    }

    for dir in dirs {
        if dir.exists() && !output.changed_files.contains(&dir) {
            fs::remove_dir_all(&dir)?;

            output.changed_files.push(dir);
        }
    }

    Ok(Json(output))
}
