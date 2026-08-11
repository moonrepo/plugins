// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::YarnToolchainConfig;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config_schema;
use moon_pdk_api::*;
use node_depman_tool::PackageManager;

#[plugin_fn]
pub fn define_requirements(
    Json(_): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    Ok(Json(DefineRequirementsOutput {
        // Nub is a standalone binary that can manage Node.js itself,
        // while the other package managers are Node.js scripts
        requires: if PackageManager::detect()?.is_nub() {
            vec![]
        } else {
            vec!["node".into()]
        },
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

        if let Some(compat_version) = &config.version {
            let compat_spec = match compat_version {
                UnresolvedVersionSpec::Range(_) | UnresolvedVersionSpec::Requirement(_) => None,
                other => Some(other.to_resolved_spec()),
            };

            if let Some(compat_spec) = compat_spec
                && PackageManager::detect_from_version(&compat_spec)? == PackageManager::Yarn2to5
            {
                for plugin in config.plugins {
                    output.commands.push(ExecCommand::new(
                        ExecCommandInput::new("yarn", ["plugin", "import", &plugin])
                            .cwd(input.root.clone()),
                    ));
                }
            }
        }
    }

    Ok(Json(output))
}
