use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: "proto".into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 42, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["up".into(), "upgrade".into()],
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/moonrepo/proto")?
        .into_iter()
        .filter_map(|tag| {
            if tag.contains("version_spec") {
                None
            } else {
                tag.strip_prefix('v').map(|tag| tag.to_owned())
            }
        })
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn build_instructions(
    Json(input): Json<BuildInstructionsInput>,
) -> FnResult<Json<BuildInstructionsOutput>> {
    let env = get_host_environment()?;
    let version = input.context.version;

    check_supported_os_and_arch(
        "proto",
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64],
        ],
    )?;

    let output = BuildInstructionsOutput {
        source: Some(SourceLocation::Archive(ArchiveSource {
            url: format!(
                "https://github.com/moonrepo/proto/releases/download/v{version}/source.tar.gz"
            ),
            prefix: Some(format!("proto_cli-{version}")),
        })),
        requirements: vec![BuildRequirement::CommandExistsOnPath("cargo".into())],
        instructions: vec![
            BuildInstruction::RunCommand(Box::new(CommandInstruction::new(
                "cargo",
                [
                    "build",
                    "--bin",
                    "proto",
                    "--bin",
                    "proto-shim",
                    "--release",
                    "--no-default-features",
                ],
            ))),
            // Move file to the root so that the executable can be located,
            // and also so that we can remove the target directory
            BuildInstruction::MoveFile(
                env.os.get_exe_name("target/release/proto").into(),
                env.os.get_exe_name("proto").into(),
            ),
            BuildInstruction::MoveFile(
                env.os.get_exe_name("target/release/proto-shim").into(),
                env.os.get_exe_name("proto-shim").into(),
            ),
            BuildInstruction::RemoveDir("target".into()),
        ],
        ..Default::default()
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        "proto",
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64],
        ],
    )?;

    let version = input.context.version;
    let arch = env.arch.to_rust_arch();

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: "proto".into()
        }));
    }

    let target = match env.os {
        HostOS::Linux => format!("{arch}-unknown-linux-{}", env.libc),
        HostOS::MacOS => format!("{arch}-apple-darwin"),
        HostOS::Windows => format!("{arch}-pc-windows-msvc"),
        _ => unreachable!(),
    };
    let target_name = format!("proto_cli-{target}");
    let target_ext = if env.os.is_windows() { "zip" } else { "tar.xz" };

    let download_file = format!("{target_name}.{target_ext}");
    let checksum_file = format!("{download_file}.sha256");
    let base_url = format!("https://github.com/moonrepo/proto/releases/download/v{version}");

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(target_name),
        checksum_url: Some(format!("{base_url}/{checksum_file}")),
        checksum_name: Some(checksum_file),
        download_url: format!("{base_url}/{download_file}"),
        download_name: Some(download_file),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    let mut primary = ExecutableConfig::new_primary(env.os.get_exe_name("proto"));
    primary.no_bin = true;
    primary.no_shim = true;

    let mut secondary = ExecutableConfig::new(env.os.get_exe_name("proto-shim"));
    secondary.no_bin = true;
    secondary.no_shim = true;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([("proto".into(), primary), ("proto-shim".into(), secondary)]),
        ..LocateExecutablesOutput::default()
    }))
}
