// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::YarnToolchainConfig;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config_schema;
use moon_pdk_api::*;
use node_depman_tool::PackageManager;
use std::path::PathBuf;

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let manager = PackageManager::detect()?;
    let mut output = SetupEnvironmentOutput::default();

    // Yarn plugins
    if manager == PackageManager::Yarn {
        let config = parse_toolchain_config_schema::<YarnToolchainConfig>(input.toolchain_config)?;

        if let Some(version) = &config.version
            && manager.is_yarn_berry(version)
        {
            for plugin in config.plugins {
                output.commands.push(ExecCommand::new(
                    ExecCommandInput::new("yarn", ["plugin", "import", &plugin])
                        .cwd(input.root.clone()),
                ));
            }
        }
    }

    Ok(Json(output))
}

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

    // TODO args requires toolchain_config in input

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
