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
        name: "uv".into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 42, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["self".into()],
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/astral-sh/uv")?;

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        "uv",
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
            tool: "uv".into()
        }));
    }

    let target = match env.os {
        HostOS::Linux => format!("{arch}-unknown-linux-{}", env.libc),
        HostOS::MacOS => format!("{arch}-apple-darwin"),
        HostOS::Windows => format!("{arch}-pc-windows-msvc"),
        _ => unreachable!(),
    };
    let target_name = format!("uv-{target}");

    let download_file = if env.os.is_windows() {
        format!("{target_name}.zip")
    } else {
        format!("{target_name}.tar.gz")
    };
    let checksum_file = format!("{download_file}.sha256");
    let base_url = format!("https://github.com/astral-sh/uv/releases/download/{version}");

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

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                "uv".into(),
                ExecutableConfig::new_primary(env.os.get_exe_name("uv")),
            ),
            (
                "uvx".into(),
                ExecutableConfig::new(env.os.get_exe_name("uvx")),
            ),
        ]),
        // https://docs.astral.sh/uv/reference/cli/#uv-tool-dir
        globals_lookup_dirs: vec![
            "$UV_TOOL_BIN_DIR".into(),
            "$XDG_BIN_HOME".into(),
            "$XDG_DATA_HOME/../bin".into(),
            "$HOME/.local/bin".into(),
        ],
        ..LocateExecutablesOutput::default()
    }))
}
