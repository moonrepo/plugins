use crate::cargo_metadata::{CargoMetadata, PackageTargetCrateType, PackageTargetKind};
use crate::cargo_toml::BaseCargoToml;
use crate::config::RustToolchainConfig;
use extism_pdk::*;
use moon_config::LanguageType;
use moon_pdk::{exec, get_host_environment, parse_toolchain_config};
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
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
            ".cargo/config".into(),
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
        default_image: Some(get_docker_image(config.version.as_ref())),
        ..Default::default()
    }))
}

// The official image has no channel tags, and a synced
// `rust-toolchain.toml` selects the exact toolchain anyways
fn get_docker_image(version: Option<&UnresolvedVersionSpec>) -> String {
    match version {
        Some(version)
            if version.is_canary()
                || version.is_alias("nightly")
                || version.get_scope() == Some("nightly") =>
        {
            "rustlang/rust:nightly".into()
        }
        Some(version) if !version.is_alias("stable") && !version.is_alias("beta") => {
            format!("rust:{}", version.to_partial_string())
        }
        _ => "rust:latest".into(),
    }
}

#[plugin_fn]
pub fn scaffold_docker(
    Json(input): Json<ScaffoldDockerInput>,
) -> FnResult<Json<ScaffoldDockerOutput>> {
    let mut output = ScaffoldDockerOutput::default();

    // Cargo requires the lib/bin target files during
    // the workspace/configs phase, which aren't copied till the
    // sources phase. Because scaffolding may attempt to run
    // Cargo commands, it will fail without these files!
    //
    // The sources phase only overlays files and can't remove them,
    // so only stub the targets that exist, otherwise an empty `main.rs`
    // becomes a binary without `fn main`.
    if input.phase == ScaffoldDockerPhase::Configs && input.project.is_some() {
        for path in get_target_stub_paths(&input.input_dir) {
            let file = input.output_dir.join(path);

            fs::write_file(&file, "")?;
            output.copied_files.push(file);
        }
    }

    Ok(Json(output))
}

fn get_target_stub_paths(input_dir: &VirtualPath) -> Vec<String> {
    // Loading from a path discovers targets on the file system, like Cargo.
    // Explicit paths outside of `src` are skipped, as Cargo doesn't
    // require them to exist when loading the manifest.
    if let Ok(manifest) = BaseCargoToml::from_path(input_dir.join("Cargo.toml")) {
        return manifest
            .lib
            .iter()
            .chain(&manifest.bin)
            .filter_map(|target| target.path.clone())
            .filter(|path| path.starts_with("src/"))
            .collect();
    }

    // Otherwise fallback to the default entry files, with
    // an empty `lib.rs` being a valid target on its own
    let has_lib = input_dir.join("src/lib.rs").exists();
    let has_main = input_dir.join("src/main.rs").exists();
    let mut paths = vec![];

    if has_lib || !has_main {
        paths.push("src/lib.rs".into());
    }

    if has_main {
        paths.push("src/main.rs".into());
    }

    paths
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let mut output = PruneDockerOutput::default();

    if !input.docker_config.delete_vendor_directories || !input.root.join("Cargo.toml").exists() {
        return Ok(Json(output));
    }

    let env = get_host_environment()?;

    // Before we can remove the target directory, we need to find it,
    // as it may be relocated, and a list of binaries to preserve
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

    if metadata.exit_code != 0 {
        return Err(anyhow!(
            "Failed to run <shell>cargo metadata</shell>: {}",
            metadata.stderr.trim()
        )
        .into());
    }

    let metadata: CargoMetadata = json::from_str(&metadata.stdout)?;

    // Directories outside of the root are not in the Docker context
    let to_root_dir = |dir: &PathBuf| {
        get_relative_path(dir, &metadata.workspace_root).map(|rel| input.root.join(rel))
    };

    let Some(target_dir) = to_root_dir(&metadata.target_directory) else {
        return Ok(Json(output));
    };

    // The `build-dir` setting splits intermediate artifacts from the
    // target directory, so remove it as well
    if let Some(build_dir) = metadata.build_directory.as_ref().and_then(to_root_dir)
        && build_dir != target_dir
        && !target_dir.starts_with(&build_dir)
        && build_dir.exists()
    {
        fs::remove_dir_all(&build_dir)?;
        output.changed_files.push(build_dir);
    }

    if !target_dir.exists() {
        return Ok(Json(output));
    }

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

    // We then need to scan each profile directory (`release`, custom
    // profiles, etc), and each target triple's profile directories, for
    // any existing binaries. Hashed artifacts in `deps` won't match.
    let mut bin_paths = vec![];

    if !bin_names.is_empty() {
        let mut profile_dirs = vec![];

        for entry in fs::read_dir(&target_dir)? {
            let dir = entry.path();

            if dir.is_dir() {
                for inner_entry in fs::read_dir(&dir)? {
                    let inner_dir = inner_entry.path();

                    if inner_dir.is_dir() {
                        profile_dirs.push(inner_dir);
                    }
                }

                profile_dirs.push(dir);
            }
        }

        for profile_dir in profile_dirs {
            for bin_name in &bin_names {
                let bin_path = profile_dir.join(bin_name);

                if bin_path.is_file()
                    && let Ok(rel_path) = bin_path.strip_prefix(&target_dir)
                {
                    bin_paths.push(rel_path.to_path_buf());
                }
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

    // If we preserved bins, rename the temp directory to the target,
    // so that other tools will find them at their original location
    if target_temp_dir.exists() {
        fs::rename(target_temp_dir, &target_dir)?;
    }

    output.changed_files.push(target_dir);

    Ok(Json(output))
}

// Paths from the host aren't split into components in WASM on Windows,
// so compare them as strings instead
fn get_relative_path(path: &Path, base: &Path) -> Option<String> {
    let rel = path.to_str()?.strip_prefix(base.to_str()?)?;
    let child = rel.trim_start_matches(['/', '\\']);

    // Must be a child, and never the base itself
    if child.is_empty() || child.len() == rel.len() {
        return None;
    }

    Some(child.replace('\\', "/"))
}
