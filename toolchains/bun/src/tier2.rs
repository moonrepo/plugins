// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::BunToolchainConfig;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let config = parse_toolchain_config::<BunToolchainConfig>(input.toolchain_config)?;

    if input.command == "bun" && !config.execute_args.is_empty() {
        output.args = Some(Extend::Prepend(config.execute_args));
    }

    if let Some(globals_dir) = input.globals_dir.and_then(|dir| dir.real_path()) {
        output.paths.push(globals_dir);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    if let Some(globals_dir) = input.globals_dir.and_then(|dir| dir.real_path()) {
        output.paths.push(globals_dir);
    }

    Ok(Json(output))
}
