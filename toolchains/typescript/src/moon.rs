use crate::config::TypeScriptConfig;
use crate::sync_project::*;
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
            "Provides sync operations that keep <file>tsconfig.json</file>'s in a healthy state."
                .into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
    }))
}

#[plugin_fn]
pub fn sync_project(Json(input): Json<SyncProjectInput>) -> FnResult<Json<SyncProjectOutput>> {
    let mut output = SyncProjectOutput::default();
    let project = load_project(input.project_id)?;

    if !is_project_toolchain_enabled(&project, "typescript") {
        output.skipped = true;

        return Ok(Json(output));
    }

    let config = get_toolchain_config::<TypeScriptConfig>(input.config)?;
    let dependencies = load_projects(input.project_dependencies)?;

    output.changed_files =
        sync_project_references(&input.context, &config, &project, &dependencies)?;

    Ok(Json(output))
}
