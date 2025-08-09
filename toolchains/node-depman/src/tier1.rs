use crate::config::*;
use extism_pdk::*;
use moon_pdk::get_plugin_id;
use moon_pdk_api::*;
use node_depman_tool::PackageManager;
use schematic::SchemaBuilder;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    let manager = PackageManager::detect()?;

    let mut output = match manager {
        PackageManager::Npm => RegisterToolchainOutput {
            config_file_globs: vec![".npmrc".into()],
            exe_names: vec!["npm".into(), "npx".into()],
            lock_file_names: vec!["package-lock.json".into()],
            ..Default::default()
        },
        PackageManager::Pnpm => RegisterToolchainOutput {
            config_file_globs: vec![
                ".npmrc".into(),
                "pnpm-workspace.yaml".into(),
                ".pnpmfile.*".into(),
            ],
            exe_names: vec!["pnpm".into(), "pnpx".into()],
            lock_file_names: vec!["pnpm-lock.yaml".into()],
            ..Default::default()
        },
        PackageManager::Yarn => RegisterToolchainOutput {
            config_file_globs: vec![".npmrc".into(), ".yarnrc.*".into()],
            exe_names: vec!["yarn".into(), "yarnpkg".into()],
            lock_file_names: vec!["yarn.lock".into()],
            ..Default::default()
        },
    };

    output.name = manager.to_string();
    output.plugin_version = env!("CARGO_PKG_VERSION").into();
    output.manifest_file_names.push("package.json".into());
    output.vendor_dir_name = Some("node_modules".into());

    Ok(Json(output))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/guides/javascript".into()),
        docs_url: Some(format!(
            "https://moonrepo.dev/docs/config/toolchain#{}",
            get_plugin_id()?
        )),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    let manager = PackageManager::detect()?;

    Ok(Json(DefineToolchainConfigOutput {
        schema: match manager {
            PackageManager::Npm => SchemaBuilder::build_root::<NpmToolchainConfig>(),
            PackageManager::Pnpm => SchemaBuilder::build_root::<PnpmToolchainConfig>(),
            PackageManager::Yarn => SchemaBuilder::build_root::<YarnToolchainConfig>(),
        },
    }))
}
