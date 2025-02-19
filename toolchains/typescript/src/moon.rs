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

    if is_project_toolchain_enabled(&input.project, "typescript") {
        let config = get_toolchain_config::<TypeScriptConfig>(input.toolchain_config)?;

        output.changed_files = sync_project_references(
            &input.context,
            &config,
            &input.project,
            &input.project_dependencies,
        )?;
    } else {
        output.skipped = true;
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn hash_task_contents(
    Json(input): Json<HashTaskContentsInput>,
) -> FnResult<Json<HashTaskContentsOutput>> {
    let mut output = HashTaskContentsOutput::default();

    Ok(Json(output))
}
