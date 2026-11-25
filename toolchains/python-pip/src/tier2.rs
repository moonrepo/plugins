// Note: Most tier 2 is implemented in the Python toolchain!

use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn define_requirements(
    Json(_): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    Ok(Json(DefineRequirementsOutput {
        // Requires python to run pip
        requires: vec![ToolchainRequirement {
            id: "unstable_python".into(),
            setup_toolchain: Some(RequirementPhase::Before),
            setup_environment: Some(RequirementPhase::Before),
        }],
    }))
}
