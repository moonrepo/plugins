use extism_pdk::*;
use proto_pdk::*;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: "moon".into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["upgrade".into()],
        ..RegisterToolOutput::default()
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
                ["build", "--bin", "moon", "--bin", "moonx", "--release"],
            ))),
            BuildInstruction::MoveFile(
                env.os.get_exe_name("target/release/moon").into(),
                env.os.get_exe_name("moon").into(),
            ),
            BuildInstruction::MoveFile(
                env.os.get_exe_name("target/release/moonx").into(),
                env.os.get_exe_name("moonx").into(),
            ),
            BuildInstruction::RemoveAllExcept(vec![
                env.os.get_exe_name("moon").into(),
                env.os.get_exe_name("moonx").into(),
            ]),
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
    let mut output = DownloadPrebuiltOutput::default();

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

    let base_url = format!(
        "https://github.com/moonrepo/moon/releases/download/{}",
        if version.is_canary() {
            "canary".to_owned()
        } else {
            format!("v{version}")
        }
    );

    let target = match env.os {
        HostOS::Linux => format!("{arch}-unknown-linux-{}", env.libc),
        HostOS::MacOS => format!("{arch}-apple-darwin"),
        HostOS::Windows => format!("{arch}-pc-windows-msvc"),
        _ => unreachable!(),
    };

    // moon v2+ binaries are published in an archive
    if is_v2(&version) {
        let target_ext = if env.os.is_windows() { "zip" } else { "tar.xz" };
        let target_name = format!("moon_cli-{target}");
        let download_file = format!("{target_name}.{target_ext}");
        let checksum_file = format!("{download_file}.sha256");

        output.archive_prefix = Some(target_name);
        output.checksum_url = Some(format!("{base_url}/{checksum_file}"));
        output.checksum_name = Some(checksum_file);
        output.download_url = format!("{base_url}/{download_file}");
        output.download_name = Some(download_file);
    }
    // moon v1 binaries are published as standalone executables
    else {
        let target_name = format!("moon-{target}");
        let download_file = if env.os.is_windows() {
            format!("{target_name}.exe")
        } else {
            target_name
        };

        output.download_url = format!("{base_url}/{download_file}");
        output.download_name = Some(download_file);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let mut output = LocateExecutablesOutput::default();

    if is_v2(&input.context.version) {
        output.exes.insert(
            "moon".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name("moon")),
        );
        output.exes.insert(
            "moonx".into(),
            ExecutableConfig::new(env.os.get_exe_name("moonx")),
        );
    } else {
        // Because moon releases do not package the binaries in archives,
        // the downloaded file gets renamed to the plugin ID, and not just "moon".
        let id = get_plugin_id()?;

        output.exes.insert(
            "moon".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name(&id)),
        );
    }

    Ok(Json(output))
}

fn is_v2(version: &VersionSpec) -> bool {
    version.as_version().is_some_and(|v| v.major >= 2)
}
