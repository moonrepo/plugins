use crate::config::JavaScriptConfig;
use extism_pdk::*;
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: "JavaScript".into(),
        description: Some(
            "Provides sync operations that keep <file>package.json</file>'s in a healthy state."
                .into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_file_globs: vec!["*.config.{js,cjs,mjs,ts,tsx,cts,mts}".into()],
        manifest_file_names: vec!["package.json".into()],
        vendor_dir_name: Some("node_modules".into()),
        ..RegisterToolchainOutput::default()
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        // config_url: Some("https://moonrepo.dev/docs/guides/rust/handbook".into()),
        // docs_url: Some("https://moonrepo.dev/docs/config/toolchain#rust".into()),
        prompts: vec![SettingPrompt::new(
            "syncToolchainConfig",
            "Sync <property>version</property> to <file>rust-toolchain.toml</file>?",
            PromptType::Confirm { default: true },
        )],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<JavaScriptConfig>(),
    }))
}
