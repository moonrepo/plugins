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
        name: "moon".into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 42, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["upgrade".into()],
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/moonrepo/moon")?
        .into_iter()
        .filter_map(|tag| tag.strip_prefix('v').map(|tag| tag.to_owned()))
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
        "moon",
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64],
        ],
    )?;

    let output = BuildInstructionsOutput {
        source: Some(SourceLocation::Archive(ArchiveSource {
            url: format!("https://github.com/moonrepo/moon/archive/refs/tags/v{version}.tar.gz"),
            prefix: Some(format!("moon-{version}")),
        })),
        requirements: vec![BuildRequirement::CommandExistsOnPath("cargo".into())],
        instructions: vec![
            BuildInstruction::RunCommand(Box::new(CommandInstruction::new(
                "cargo",
                ["build", "--bin", "moon", "--release"],
            ))),
            // Move file to the root so that the executable can be located,
            // and also so that we can remove the target directory
            BuildInstruction::MoveFile(
                env.os.get_exe_name("target/release/moon").into(),
                env.os.get_exe_name("moon").into(),
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
        "moon",
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64],
        ],
    )?;

    let version = input.context.version;
    let arch = env.arch.to_rust_arch();

    let tag = if version.is_canary() {
        "canary".to_owned()
    } else {
        format!("v{version}")
    };

    let target = match env.os {
        HostOS::Linux => format!("{arch}-unknown-linux-{}", env.libc),
        HostOS::MacOS => format!("{arch}-apple-darwin"),
        HostOS::Windows => format!("{arch}-pc-windows-msvc"),
        _ => unreachable!(),
    };
    let target_name = format!("moon-{target}");

    let download_file = if env.os.is_windows() {
        format!("{target_name}.exe")
    } else {
        target_name
    };
    let base_url = format!("https://github.com/moonrepo/moon/releases/download/{tag}");

    Ok(Json(DownloadPrebuiltOutput {
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

    // Because moon releases do not package the binaries in archives,
    // the downloaded file gets renamed to the plugin ID, and not just "moon".
    let id = get_plugin_id()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "moon".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name(id)),
        )]),
        ..LocateExecutablesOutput::default()
    }))
}
