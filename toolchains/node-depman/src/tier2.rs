// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::YarnToolchainConfig;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config_schema;
use moon_pdk_api::*;
use node_depman_tool::PackageManager;
use proto_pdk_api::UnresolvedVersionSpec;

#[plugin_fn]
pub fn define_requirements(
    Json(_): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    Ok(Json(DefineRequirementsOutput {
        requires: vec!["node".into()],
    }))
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let manager = PackageManager::detect()?;
    let mut output = SetupEnvironmentOutput::default();

    // Yarn plugins
    if manager.is_yarn() {
        let config = parse_toolchain_config_schema::<YarnToolchainConfig>(input.toolchain_config)?;

        // TODO fix once moon is on proto 0.59
        if let Some(incompat_version) = &config.version {
            let _compat_version = UnresolvedVersionSpec::parse(incompat_version.to_string())?;

            // if manager.is_yarn_berry(&compat_version) {
            for plugin in config.plugins {
                output.commands.push(ExecCommand::new(
                    ExecCommandInput::new("yarn", ["plugin", "import", &plugin])
                        .cwd(input.root.clone()),
                ));
            }
            //}
        }
    }

    Ok(Json(output))
}
