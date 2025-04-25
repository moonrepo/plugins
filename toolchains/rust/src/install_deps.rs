use crate::cargo_toml::CargoToml;
use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let mut output = LocateDependenciesRootOutput::default();

    // Attempt to find `Cargo.lock` first
    let mut dir = input.starting_dir.clone();

    while !dir.to_str().is_some_and(|d| d == "/") {
        if !dir.join("Cargo.lock").exists() {
            dir = dir.parent();
            continue;
        }

        output.root = Some(dir.to_owned());

        let manifest_path = dir.join("Cargo.toml");

        if manifest_path.exists() {
            output.members = CargoToml::load(manifest_path)?.extract_members();
        }
    }

    // Otherwise find a `Cargo.toml` workspace
    if output.root.is_none() {
        let mut dir = input.starting_dir.clone();

        while !dir.to_str().is_some_and(|d| d == "/") {
            let manifest_path = dir.join("Cargo.toml");

            if !manifest_path.exists() {
                dir = dir.parent();
                continue;
            }

            let manifest = CargoToml::load(manifest_path)?;

            if manifest.workspace.is_some() {
                output.root = Some(dir.to_owned());
                output.members = manifest.extract_members();
                break;
            }
        }
    }

    // Else the current directory may be a stand-alone project
    if output.root.is_none() && input.starting_dir.join("Cargo.toml").exists() {
        output.root = Some(input.starting_dir);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let mut output = InstallDependenciesOutput::default();

    // Cargo does not support an "install dependencies" command
    // as it automatically happens when running any Cargo commands.
    // However, if we don't detect a lockfile, we can attempt
    // to generate one!
    if !input.root.join("Cargo.lock").exists() {
        output.install_command = Some(ExecCommandInput::pipe("cargo", ["generate-lockfile"]));
    }

    Ok(Json(output))
}
