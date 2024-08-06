use crate::config::NodeConfig;
use extism_pdk::*;
use moon_pdk::*;
use schematic::SchemaBuilder;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<ToolchainMetadataInput>,
) -> FnResult<Json<ToolchainMetadataOutput>> {
    Ok(Json(ToolchainMetadataOutput {
        config_schema: Some(SchemaBuilder::build_root::<NodeConfig>()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        ..ToolchainMetadataOutput::default()
    }))
}
