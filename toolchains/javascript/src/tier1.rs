use crate::config::*;
use crate::package_json::PackageJson;
use extism_pdk::*;
use moon_common::path::{is_root_level_source, to_relative_virtual_string};
use moon_config::DependencyScope;
use moon_pdk::{
    HostLogInput, host_log, is_project_toolchain_enabled, map_miette_error,
    parse_toolchain_config_schema, plugin_err,
};
use moon_pdk_api::*;
use nodejs_package_json::VersionProtocol;
use schematic::SchemaBuilder;
use starbase_utils::json::JsonValue;
use std::str::FromStr;
use toolchain_common::enable_tracing;

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
        // For project detection
        config_file_globs: vec![
            "*.config.{js,cjs,mjs,ts,tsx,cts,mts}".into(),
            // bun
            "bunfig.toml".into(),
            // npm
            ".npmrc".into(),
            // pnpm
            "pnpm-workspace.yaml".into(),
            ".pnpmfile.*".into(),
            // yarn
            ".yarnrc.*".into(),
        ],
        manifest_file_names: vec!["package.json".into()],
        lock_file_names: vec![
            // bun
            "bun.lock".into(),
            "bun.lockb".into(),
            // npm
            "package-lock.json".into(),
            "npm-shrinkwrap.json".into(),
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
            // node
            "node".into(),
            "nodejs".into(),
            // npm
            "npm".into(),
            "npx".into(),
            // pnpm
            "pnpm".into(),
            "pnpx".into(),
            // yarn
            "yarn".into(),
            "yarnpkg".into()
        ],
        vendor_dir_name: Some("node_modules".into()),
        ..RegisterToolchainOutput::default()
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(input): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    let mut output = InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/guides/javascript".into()),
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
                default_index: 1,
                options: vec![
                    JsonValue::String("bun".into()),
                    JsonValue::String("npm".into()),
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

    if let Some(pm) = &package.package_manager {
        let pm_name = pm.split_once('@').map(|parts| parts.0).unwrap_or(pm);

        return Ok(Some(JavaScriptPackageManager::from_str(pm_name)?));
    }

    if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
        return Ok(Some(JavaScriptPackageManager::Bun));
    } else if root.join("package-lock.json").exists() || root.join("npm-shrinkwrap.json").exists() {
        return Ok(Some(JavaScriptPackageManager::Npm));
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

    // Does not apply to root projects
    if !is_project_toolchain_enabled(&input.project) || is_root_level_source(&input.project.source)
    {
        output.skipped = true;

        return Ok(Json(output));
    }

    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config.clone())?;
    let mut package = PackageJson::load(
        input
            .context
            .get_project_root(&input.project)
            .join("package.json"),
    )?;

    // Enforce single version policy
    if config.root_package_dependencies_only
        && (package.dependencies.is_some()
            || package.dev_dependencies.is_some()
            || package.peer_dependencies.is_some()
            || package.optional_dependencies.is_some())
    {
        return Err(plugin_err!(
            "Dependencies can only be defined in the root <file>package.json</file>, found dependencies in project <id>{}</id>.\nThis is enforced through the <property>javascript.rootPackageDependenciesOnly</property> toolchain setting.",
            input.project.id
        ));
    }

    // Sync workspace dependencies
    if config.sync_project_workspace_dependencies
        && let Some(package_manager) = &config.package_manager
    {
        let (op, _) = Operation::track("sync-project-workspace-dependencies", || {
            sync_project_workspace_dependencies(&input, &config, &mut package, package_manager)
        })?;

        output.operations.push(op);

        if let Some(file) = package.save()?
            && let Some(virtual_file) = file.virtual_path()
        {
            output.changed_files.push(virtual_file);
        }
    }

    Ok(Json(output))
}

fn sync_project_workspace_dependencies(
    input: &SyncProjectInput,
    config: &JavaScriptToolchainConfig,
    package: &mut PackageJson,
    package_manager: &JavaScriptPackageManager,
) -> AnyResult<()> {
    let project_root = input.context.get_project_root(&input.project);
    let version_format = config
        .dependency_version_format
        .get_supported_for(package_manager);
    let version_prefix = version_format.get_prefix();

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

        let dep_project_root = input.context.get_project_root(dep_project);
        let dep_package_path = dep_project_root.join("package.json");

        // Only sync if a package exists
        if !dep_package_path.exists() {
            continue;
        }

        let dep_package = PackageJson::load(dep_package_path)?;

        if let Some(name) = dep_package.name.as_ref()
            && let Some(version) = dep_package.version.as_ref()
            && let Some(scope) = dep_project.dependency_scope.as_ref()
        {
            let protocol = match version_format {
                JavaScriptDependencyVersionFormat::File
                | JavaScriptDependencyVersionFormat::Link => VersionProtocol::try_from(format!(
                    "{version_prefix}{}",
                    to_relative_virtual_string(dep_project_root, &project_root)
                        .map_err(map_miette_error)?
                ))?,
                JavaScriptDependencyVersionFormat::Version
                | JavaScriptDependencyVersionFormat::VersionCaret
                | JavaScriptDependencyVersionFormat::VersionTilde => {
                    VersionProtocol::try_from(format!("{version_prefix}{version}"))?
                }
                _ => VersionProtocol::from_str(&version_prefix)?,
            };

            match scope {
                DependencyScope::Build | DependencyScope::Root => {
                    // Not used
                }
                DependencyScope::Development => {
                    package.add_dev_dependency(name, protocol, true)?;
                }
                DependencyScope::Production => {
                    package.add_dependency(name, protocol, true)?;
                }
                DependencyScope::Peer => {
                    package.add_peer_dependency(
                        name,
                        VersionProtocol::try_from(format!("^{}.0.0", version.major))?,
                        true,
                    )?;
                }
            };

            host_log!(
                "Syncing <id>{}</file> as a dependency to <id>{}</id>'s <file>package.json</file>",
                dep_project.id,
                input.project.id,
            );
        }
    }

    Ok(())
}
