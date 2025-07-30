use extism_pdk::*;
use moon_pdk_api::*;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        // config_schema: Some(SchemaBuilder::build_root::<NodeConfig>()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        ..RegisterToolchainOutput::default()
    }))
}
