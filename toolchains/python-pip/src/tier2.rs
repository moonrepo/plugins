// Note: Most tier 2 is implemented in the Python toolchain!

use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn define_requirements(
    Json(_): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    Ok(Json(DefineRequirementsOutput {
        requires: vec!["unstable_python".into()],
        for_setup_environment: false,
        // Requires python to run pip
        for_setup_toolchain: true,
    }))
}
