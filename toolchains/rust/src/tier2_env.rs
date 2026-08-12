use crate::cargo_toml::CargoToml;
use crate::config::RustToolchainConfig;
use crate::toolchain_toml::ToolchainToml;
use extism_pdk::*;
use moon_config::BinEntry;
use moon_pdk::{get_host_environment, parse_toolchain_config_schema};
use moon_pdk_api::*;
use starbase_utils::fs;

fn create_command(bin: &str, args: Vec<&str>, cwd: &VirtualPath) -> ExecCommand {
    ExecCommand::new(ExecCommandInput::new(bin, args).cwd(cwd.to_owned()))
}

fn is_bin_installed(env: &HostEnvironment, globals_dir: Option<&VirtualPath>, spec: &str) -> bool {
    let Some(globals_dir) = globals_dir else {
        return false;
    };

    // Entries may be suffixed with a version: `cargo-nextest@0.9.52`
    let name = match spec.split_once('@') {
        Some((prefix, _)) => prefix,
        None => spec,
    };

    globals_dir.join(env.os.get_exe_name(name)).exists()
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let config = parse_toolchain_config_schema::<RustToolchainConfig>(input.toolchain_config)?;
    let mut output = SetupEnvironmentOutput::default();

    // Sync `Cargo.toml` rust version
    if config.add_msrv_constraint && config.version.is_some() {
        let (op, files) = Operation::track("add-msrv-constraint", || {
            sync_package_msrv(&config, &input.root)
        })?;

        output.operations.push(op);
        output.changed_files.extend(files);
    }

    // Sync `rust-toolchain.toml` toolchain
    if config.sync_toolchain_config {
        let (op, files) = Operation::track("sync-toolchain-config", || {
            sync_toolchain_toml(&config, &input.root)
        })?;

        output.operations.push(op);
        output.changed_files.extend(files);
    }

    // Install components
    if !config.components.is_empty() {
        let mut args = vec!["component", "add"];
        args.extend(config.components.iter().map(|c| c.as_str()));

        output.commands.push(
            create_command("rustup", args, &input.root)
                .cache(CacheStrategy::Memory)
                .label("rustup-component-add"),
        );
    }

    // Install targets
    if !config.targets.is_empty() {
        let mut args = vec!["target", "add"];
        args.extend(config.targets.iter().map(|c| c.as_str()));

        output.commands.push(
            create_command("rustup", args, &input.root)
                .cache(CacheStrategy::Memory)
                .label("rustup-target-add"),
        );
    }

    // Install binaries
    if !config.bins.is_empty() {
        let env = get_host_environment()?;

        let mut force_bins = vec![];
        let mut non_force_bins = vec![];

        for bin in &config.bins {
            match bin {
                BinEntry::String(inner) => {
                    if !is_bin_installed(env, input.globals_dir.as_ref(), inner) {
                        non_force_bins.push(inner.as_str());
                    }
                }
                BinEntry::Object(cfg) => {
                    if cfg.local && env.ci {
                        continue;
                    } else if cfg.force {
                        force_bins.push(cfg.bin.as_str());
                    } else if !is_bin_installed(env, input.globals_dir.as_ref(), &cfg.bin) {
                        non_force_bins.push(cfg.bin.as_str());
                    }
                }
            };
        }

        if !force_bins.is_empty() || !non_force_bins.is_empty() {
            // Only install if we can't find the binary
            if input
                .globals_dir
                .as_ref()
                .is_none_or(|dir| !dir.join(env.os.get_exe_name("cargo-binstall")).exists())
            {
                let binstall_package = if let Some(version) = &config.binstall_version {
                    format!("cargo-binstall@{version}")
                } else {
                    "cargo-binstall".into()
                };

                output.commands.push(
                    create_command(
                        "cargo",
                        vec!["install", &binstall_package, "--force", "--locked"],
                        &input.root,
                    )
                    .cache(CacheStrategy::Memory)
                    .label("cargo-binstall"),
                );
            }

            if !force_bins.is_empty() {
                let mut args = vec!["binstall", "--no-confirm", "--log-level", "info", "--force"];
                args.extend(force_bins);

                output.commands.push(
                    create_command("cargo", args, &input.root)
                        .cache(CacheStrategy::Memory)
                        .label("cargo-bins-forced"),
                );
            }

            if !non_force_bins.is_empty() {
                let mut args = vec!["binstall", "--no-confirm", "--log-level", "info"];
                args.extend(non_force_bins);

                output.commands.push(
                    create_command("cargo", args, &input.root)
                        .cache(CacheStrategy::Memory)
                        .label("cargo-bins"),
                );
            }
        }
    }

    Ok(Json(output))
}

fn sync_package_msrv(
    config: &RustToolchainConfig,
    root: &VirtualPath,
) -> AnyResult<Vec<VirtualPath>> {
    let mut changed_files = vec![];
    let manifest_path = root.join("Cargo.toml");

    if let Some(version) = &config.version
        && manifest_path.exists()
    {
        let mut manifest = CargoToml::load(manifest_path)?;
        manifest.set_msrv(version.to_string())?;

        if let Some(file) = manifest.save()? {
            changed_files.push(file);
        }
    }

    Ok(changed_files)
}

fn sync_toolchain_toml(
    config: &RustToolchainConfig,
    root: &VirtualPath,
) -> AnyResult<Vec<VirtualPath>> {
    let mut changed_files = vec![];
    let legacy_toolchain_path = root.join("rust-toolchain");
    let toolchain_path = root.join("rust-toolchain.toml");

    // Convert `rust-toolchain` to `rust-toolchain.toml`
    if legacy_toolchain_path.exists() {
        let legacy_contents = fs::read_file(&legacy_toolchain_path)?;

        if legacy_contents.contains("[toolchain]") {
            fs::rename(&legacy_toolchain_path, &toolchain_path)?;
        } else {
            fs::remove_file(&legacy_toolchain_path)?;
            fs::write_file(
                &toolchain_path,
                format!("[toolchain]\nchannel = \"{}\"", legacy_contents.trim()),
            )?;
        }

        changed_files.push(toolchain_path.clone());
        changed_files.push(legacy_toolchain_path);
    }

    if let Some(version) = &config.version {
        let mut contents = ToolchainToml::load(toolchain_path)?;
        contents.set_channel(version.to_string())?;

        if let Some(file) = contents.save()?
            && !changed_files.contains(&file)
        {
            changed_files.push(file);
        }
    }

    Ok(changed_files)
}
