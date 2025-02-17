use crate::config::TypeScriptConfig;
use extism_pdk::*;
use moon_pdk::*;
use schematic::SchemaBuilder;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        config_schema: Some(SchemaBuilder::build_root::<TypeScriptConfig>()),
        name: "TypeScript".into(),
        description: Some(
            "Provides sync operation that keep <file>tsconfig.json</file> in a healthy state."
                .into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        ..RegisterToolchainOutput::default()
    }))
}
