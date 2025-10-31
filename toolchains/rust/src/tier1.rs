use crate::cargo_metadata::{CargoMetadata, PackageTargetCrateType, PackageTargetKind};
use crate::config::RustToolchainConfig;
use extism_pdk::*;
use moon_config::LanguageType;
use moon_pdk::{exec, get_host_environment, parse_toolchain_config};
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use std::path::PathBuf;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: "Rust".into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        language: Some(LanguageType::Rust),
        config_file_globs: vec![
            ".cargo/*.toml".into(),
            "rust-toolchain".into(),
            "rust-toolchain.toml".into(),
        ],
        exe_names: vec![
            "cargo".into(),
            "rustc".into(),
            "rustdoc".into(),
            "rustfmt".into(),
            "rustup".into(),
        ],
        lock_file_names: vec!["Cargo.lock".into()],
        manifest_file_names: vec!["Cargo.toml".into()],
        // proto_tool_id: Some("rust".into()),
        vendor_dir_name: Some("target".into()),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/guides/rust/handbook".into()),
        docs_url: Some("https://moonrepo.dev/docs/config/toolchain#rust".into()),
        prompts: vec![SettingPrompt::new(
            "syncToolchainConfig",
            "Sync <property>version</property> to <file>rust-toolchain.toml</file>?",
            PromptType::Confirm { default: true },
        )],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<RustToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    let config = parse_toolchain_config::<RustToolchainConfig>(input.toolchain_config)?;

    Ok(Json(DefineDockerMetadataOutput {
        default_image: Some(format!(
            "rust:{}",
            config
                .version
                .as_ref()
                .map(|version| version.to_partial_string())
                .unwrap_or_else(|| "latest".into())
        )),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn scaffold_docker(
    Json(input): Json<ScaffoldDockerInput>,
) -> FnResult<Json<ScaffoldDockerOutput>> {
    let mut output = ScaffoldDockerOutput::default();

    // Cargo requires either `lib.rs` or `main.rs` during
    // the workspace/configs phase, which isn't copied till the
    // sources phase. Because scaffolding may attempt to run
    // Cargo commands, it will fail without these files!
    if input.phase == ScaffoldDockerPhase::Configs && input.project.is_some() {
        let lib_file = input.output_dir.join("src/lib.rs");
        let main_file = input.output_dir.join("src/main.rs");

        fs::write_file(&lib_file, "")?;
        fs::write_file(&main_file, "")?;

        if let Some(file) = lib_file.virtual_path() {
            output.copied_files.push(file);
        }

        if let Some(file) = main_file.virtual_path() {
            output.copied_files.push(file);
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let mut output = PruneDockerOutput::default();
    let target_dir = input.root.join("target");

    if !target_dir.exists() || !input.docker_config.delete_vendor_directories {
        return Ok(Json(output));
    }

    let env = get_host_environment()?;

    // Before we can remove the target directory, we need
    // to find a list of binaries to preserve
    let metadata = exec(
        ExecCommandInput::pipe(
            "cargo",
            [
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--no-default-features",
            ],
        )
        .cwd(input.root.clone()),
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

    if let Some(file) = target_dir.virtual_path() {
        output.changed_files.push(file);
    }

    // If we preserved bins, rename the temp directory to the target,
    // so that other tools will find them at their original location
    if target_temp_dir.exists() {
        fs::rename(target_temp_dir, target_dir)?;
    }

    Ok(Json(output))
}
