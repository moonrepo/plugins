use crate::config::TypeScriptConfig;
use crate::run_task::*;
use crate::sync_project::*;
use crate::tsconfig_json::TsConfigJson;
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
    let config = get_toolchain_config::<TypeScriptConfig>(input.toolchain_config)?;
    let mut output = HashTaskContentsOutput::default();

    for tsconfig_path in [
        input
            .context
            .workspace_root
            .join(&config.root)
            .join(&config.root_config_file_name),
        input
            .context
            .workspace_root
            .join(&input.project.source)
            .join(&config.project_config_file_name),
    ] {
        if tsconfig_path.exists() {
            let tsconfig = TsConfigJson::load(tsconfig_path)?;

            if let Some(options) = &tsconfig.compiler_options {
                output.contents.push(hash_compiler_options(options));
            }
        }
    }

    Ok(Json(output))
}
