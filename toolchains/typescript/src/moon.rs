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
        name: "TypeScript".into(),
        description: Some(
            "Provides sync operations that keep <file>tsconfig.json</file>'s in a healthy state."
                .into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_file_globs: vec![
            "tsconfig.json".into(),
            "tsconfig.*.json".into(),
            "*.tsconfig.json".into(),
            ".tsbuildinfo".into(),
            "*.tsbuildinfo".into(),
        ],
        config_schema: Some(SchemaBuilder::build_root::<TypeScriptConfig>()),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn sync_project(Json(input): Json<SyncProjectInput>) -> FnResult<Json<SyncOutput>> {
    let plugin_id = get_plugin_id()?;
    let mut output = SyncOutput::default();

    if is_project_toolchain_enabled(&input.project, &plugin_id) {
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
            let tsconfig = TsConfigJson::load_with_extends(tsconfig_path)?;

            if let Some(options) = &tsconfig.compiler_options {
                let data = hash_compiler_options(options);

                if data.as_object().is_some_and(|obj| !obj.is_empty()) {
                    output.contents.push(data);
                }
            }
        }
    }

    Ok(Json(output))
}
