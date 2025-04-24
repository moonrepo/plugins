use extism_pdk::*;
use moon_pdk_api::*;
use starbase_utils::fs;

#[plugin_fn]
pub fn scaffold_docker(
    Json(input): Json<ScaffoldDockerInput>,
) -> FnResult<Json<ScaffoldDockerOutput>> {
    let mut output = ScaffoldDockerOutput::default();

    // Cargo requires either `lib.rs` or `main.rs` during
    // the workspace/configs phase, which isn't copied till the
    // sources phase. Because scaffolding may attempt to run
    // Cargo commands, it will fail without these files!
    if input.phase == ScaffoldDockerPhase::Configs {
        let lib_file = input.output_dir.join("src/lib.rs");
        let main_file = input.output_dir.join("src/main.rs");

        fs::write_file(&lib_file, "");
        fs::write_file(&main_file, "");

        output.copied_files.push(lib_file);
        output.copied_files.push(main_file);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn prune_docker(Json(_input): Json<PruneDockerInput>) -> FnResult<()> {
    // let mut output = ScaffoldDockerOutput::default();

    // TODO remove target dir

    Ok(())
}
