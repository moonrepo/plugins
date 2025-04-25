use extism_pdk::*;
use moon_pdk_api::*;
use starbase_utils::fs;

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let mut output = SetupEnvironmentOutput::default();

    Ok(Json(output))
}
