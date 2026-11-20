// Note: Most tier 2 is implemented in the JavaScript toolchain!

use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn define_requirements(
    Json(_): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    Ok(Json(DefineRequirementsOutput {
        requires: vec!["node".into()],
        for_setup_environment: false,
        for_setup_toolchain: true,
    }))
}
