use crate::config::{JavaScriptPackageManager, JavaScriptToolchainConfig};
use crate::package_json::PackageJson;
use extism_pdk::*;
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use starbase_utils::json::JsonValue;
use std::str::FromStr;
use toolchain_common::enable_tracing;

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

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<JavaScriptToolchainConfig>(),
    }))
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
    } else if root.join("pnpm-lock.yaml").exists() {
        return Ok(Some(JavaScriptPackageManager::Pnpm));
    } else if root.join("yarn.lock").exists() {
        return Ok(Some(JavaScriptPackageManager::Yarn));
    }

    Ok(None)
}
