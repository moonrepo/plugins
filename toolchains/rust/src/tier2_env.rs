use crate::binstall::{get_binstall_archive, get_binstall_url};
use crate::cargo_toml::CargoToml;
use crate::config::RustToolchainConfig;
use crate::tier2::get_pinned_toolchain;
use crate::toolchain_toml::ToolchainToml;
use crate::version::{to_channel, to_msrv};
use extism_pdk::*;
use moon_config::BinEntry;
use moon_pdk::{exec, fetch_bytes, get_host_environment, host_log, parse_toolchain_config_schema};
use moon_pdk_api::*;
use starbase_utils::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

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

    // Target the configured toolchain, not the default
    let toolchain = get_pinned_toolchain(&config)?;
    let toolchain_args = match &toolchain {
        Some(channel) => vec!["--toolchain", channel.as_str()],
        None => vec![],
    };

    // Install components
    if !config.components.is_empty() {
        let mut args = vec!["component", "add"];
        args.extend(&toolchain_args);
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
        args.extend(&toolchain_args);
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
        let mut args_bins = vec![];

        for bin in &config.bins {
            match bin {
                BinEntry::String(inner) => {
                    if !is_bin_installed(env, input.globals_dir.as_ref(), inner) {
                        non_force_bins.push(inner.as_str());
                    }
                }
                BinEntry::Object(cfg) => {
                    if (cfg.local && env.ci)
                        || (!cfg.force
                            && is_bin_installed(env, input.globals_dir.as_ref(), &cfg.bin))
                    {
                        continue;
                    }

                    if !cfg.args.is_empty() {
                        args_bins.push(cfg);
                    } else if cfg.force {
                        force_bins.push(cfg.bin.as_str());
                    } else {
                        non_force_bins.push(cfg.bin.as_str());
                    }
                }
            };
        }

        if !force_bins.is_empty() || !non_force_bins.is_empty() || !args_bins.is_empty() {
            // Only install if we can't find the binary
            if input
                .globals_dir
                .as_ref()
                .is_none_or(|dir| !dir.join(env.os.get_exe_name("cargo-binstall")).exists())
            {
                let mut installed = false;

                if let Some(globals_dir) = &input.globals_dir
                    && let Some(archive) = get_binstall_archive(env.os, env.arch)
                {
                    let (op, result) =
                        Operation::track_with_error("install-cargo-binstall", || {
                            install_prebuilt_binstall(
                                env,
                                globals_dir,
                                &archive,
                                config.binstall_version.as_ref(),
                            )
                        })?;

                    installed = result.is_some();
                    output.operations.push(op);
                }

                // Otherwise compile it from source
                if !installed {
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

            // Args apply to all crates in a command, so install these individually
            for cfg in args_bins {
                let mut args = vec!["binstall", "--no-confirm", "--log-level", "info"];

                if cfg.force {
                    args.push("--force");
                }

                args.extend(cfg.args.iter().map(|arg| arg.as_str()));
                args.push(&cfg.bin);

                output.commands.push(
                    create_command("cargo", args, &input.root)
                        .cache(CacheStrategy::Memory)
                        .label(format!("cargo-bin-{}", cfg.bin)),
                );
            }
        }
    }

    Ok(Json(output))
}

// `cargo install cargo-binstall` compiles from source, which takes minutes on
// a cold CI runner, so install the prebuilt release, like binstall's own installer
fn install_prebuilt_binstall(
    env: &HostEnvironment,
    globals_dir: &VirtualPath,
    archive: &str,
    version: Option<&Version>,
) -> AnyResult<()> {
    let temp_dir = VirtualPath::new("/moon/temp/rust").join(format!(
        "cargo-binstall-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));

    fs::create_dir_all(&temp_dir)?;

    let result = unpack_prebuilt_binstall(env, globals_dir, &temp_dir, archive, version);

    // Leftovers in the temp dir are harmless, so don't fail an install
    let _ = fs::remove_dir_all(&temp_dir);

    result
}

fn unpack_prebuilt_binstall(
    env: &HostEnvironment,
    globals_dir: &VirtualPath,
    temp_dir: &VirtualPath,
    archive: &str,
    version: Option<&Version>,
) -> AnyResult<()> {
    let url = get_binstall_url(archive, version);

    host_log!(
        debug,
        "Downloading prebuilt <shell>cargo-binstall</shell> from <url>{url}</url>",
    );

    fs::write_file(temp_dir.join(archive), fetch_bytes(&url)?)?;

    // GNU tar reads `.tgz`, while bsdtar (macOS, Windows 10+) also reads `.zip`
    let unpack = exec(ExecCommandInput::pipe("tar", ["-xf", archive]).cwd(temp_dir.to_owned()))?;

    if unpack.exit_code != 0 {
        return Err(anyhow!(
            "Failed to unpack <file>{archive}</file>: {}",
            unpack.stderr.trim()
        ));
    }

    // WASI can't set permissions, so copy next to the final path, and let
    // the host make it executable while checking that it runs
    let exe_name = env.os.get_exe_name("cargo-binstall");
    let bin_path = globals_dir.join(&exe_name);
    let partial_path = globals_dir.join(env.os.get_exe_name("cargo-binstall-partial"));

    fs::copy_file(temp_dir.join(&exe_name), &partial_path)?;

    let mut check = ExecCommandInput::pipe(partial_path.to_string(), ["-V"]);
    check.set_executable = true;

    let check = exec(check)?;

    if check.exit_code != 0 {
        fs::remove_file(&partial_path)?;

        return Err(anyhow!(
            "Prebuilt <shell>cargo-binstall</shell> failed to run: {}",
            check.stderr.trim()
        ));
    }

    fs::rename(&partial_path, &bin_path)?;

    Ok(())
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
        let Some(msrv) = to_msrv(version) else {
            host_log!(
                warn,
                "Unable to add an MSRV constraint, as <version>{version}</version> is not a valid <property>rust-version</property> (expected 1.80 or 1.80.0)",
            );

            return Ok(changed_files);
        };

        let mut manifest = CargoToml::load(manifest_path)?;
        manifest.set_msrv(msrv)?;

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

    let channel = config.version.as_ref().and_then(to_channel);

    if let Some(version) = &config.version
        && channel.is_none()
    {
        host_log!(
            warn,
            "Unable to sync <property>channel</property> to <file>rust-toolchain.toml</file>, as <version>{version}</version> is not a valid channel (expected stable, beta, nightly, or 1.80.0)",
        );
    }

    // Only create the file when there's a channel to pin
    if channel.is_none()
        && (!toolchain_path.exists() || (config.components.is_empty() && config.targets.is_empty()))
    {
        return Ok(changed_files);
    }

    // Saving merges into the existing file, so it must exist
    if !toolchain_path.exists() {
        fs::write_file(&toolchain_path, "[toolchain]\n")?;
    }

    // Rustup installs missing components and targets for whichever
    // channel the file uses, so editors and ad-hoc `cargo` get them too
    let mut contents = ToolchainToml::load(toolchain_path)?;

    if let Some(channel) = channel {
        contents.set_channel(channel)?;
    }

    contents.add_components(&config.components)?;
    contents.add_targets(&config.targets)?;

    if let Some(file) = contents.save()?
        && !changed_files.contains(&file)
    {
        changed_files.push(file);
    }

    Ok(changed_files)
}
