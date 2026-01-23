use crate::config::*;
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
        name: "pip".into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_file_globs: vec!["pip.conf".into(), "pip.ini".into()],
        exe_names: vec!["pip".into()],
        manifest_file_names: vec!["pyproject.toml".into(), "requirements.in".into()],
        lock_file_names: vec![
            "pylock.toml".into(),
            "requirements.txt".into(),
            "constraints.txt".into(),
        ],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<PipToolchainConfig>(),
    }))
}
