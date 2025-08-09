// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::BunToolchainConfig;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;
use std::path::PathBuf;

fn gather_shared_paths(
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(globals_dir) = globals_dir {
        if let Some(value) = globals_dir.real_path() {
            paths.push(value);
        }
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let config = parse_toolchain_config::<BunToolchainConfig>(input.toolchain_config)?;

    if input.command == "bun" && !config.execute_args.is_empty() {
        output.args = Some(Extend::Prepend(config.execute_args));
    }

    gather_shared_paths(input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    gather_shared_paths(input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}
