use std::path::PathBuf;

use crate::cargo_metadata::{CargoMetadata, PackageTargetCrateType, PackageTargetKind};
use extism_pdk::*;
use moon_pdk::{exec_captured, get_host_environment};
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

        fs::write_file(&lib_file, "")?;
        fs::write_file(&main_file, "")?;

        output.copied_files.push(lib_file);
        output.copied_files.push(main_file);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let mut output = PruneDockerOutput::default();
    let target_dir = input.root.join("target");

    if !target_dir.exists() {
        return Ok(Json(output));
    }

    let env = get_host_environment()?;

    // Before we can remove the target directory, we need
    // to find a list of binaries to preserve
    let metadata = exec_captured(
        "cargo",
        ["metadata --format-version 1 --no-deps --no-default-features"],
    )?;
    let metadata: CargoMetadata = json::from_str(&metadata.stdout)?;
    let mut bin_names = vec![];

    for package in metadata.packages {
        for target in package.targets {
            if target.crate_types.contains(&PackageTargetCrateType::Bin)
                && target.kind.contains(&PackageTargetKind::Bin)
            {
                bin_names.push(env.os.get_exe_name(target.name));
            }
        }
    }

    // We then need to scan the target directory and each
    // build profile for any existing binaries
    let mut bin_paths = vec![];

    for bin_name in bin_names {
        for profile_name in ["release", "debug"] {
            let bin_path = PathBuf::from(profile_name).join(&bin_name);

            if target_dir.join(&bin_path).exists() {
                bin_paths.push(bin_path);
            }
        }
    }

    // If found, preserve them by moving to another folder
    let target_temp_dir = input.root.join("target-temp");

    for bin_path in bin_paths {
        fs::rename(target_dir.join(&bin_path), target_temp_dir.join(&bin_path))?;
    }

    // We can now delete the target directory, this may take a while...
    fs::remove_dir_all(&target_dir)?;

    output.changed_files.push(target_dir.clone());

    // If we preserved bins, rename the temp directory to the target,
    // so that other tools will find them at their original location
    if target_temp_dir.exists() {
        fs::rename(target_temp_dir, target_dir)?;
    }

    Ok(Json(output))
}
