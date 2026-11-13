use extism_pdk::*;
use proto_pdk::*;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "Pkl";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 64, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    // Release tags are plain versions, without a `v` prefix
    let tags = load_git_tags("https://github.com/apple/pkl")?
        .into_iter()
        .filter(|tag| Version::parse(tag).is_ok())
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        NAME,
        env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64],
        ],
    )?;

    let version = input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(
            "{NAME} does not publish canary builds. Use a version or the latest alias instead."
        ));
    }

    let at_least = |major, minor, patch| {
        version
            .as_version()
            .is_some_and(|v| *v >= Version::new(major, minor, patch))
    };

    let unsupported = match (env.os, env.arch) {
        // Not published for musl
        (HostOS::Linux, HostArch::Arm64) => env.libc == HostLibc::Musl,
        // Dropped in v0.33
        (HostOS::MacOS, HostArch::X64) => at_least(0, 33, 0),
        // First published in v0.26
        (HostOS::Windows, _) => !at_least(0, 26, 0),
        _ => false,
    };

    if unsupported {
        return Err(plugin_err!(PluginError::UnsupportedTarget {
            tool: NAME.into(),
            arch: env.arch.to_string(),
            os: env.os.to_string(),
        }));
    }

    let arch = match env.arch {
        HostArch::Arm64 => "aarch64",
        _ => "amd64",
    };

    let download_file = match env.os {
        HostOS::Linux if env.libc == HostLibc::Musl => format!("pkl-alpine-linux-{arch}"),
        HostOS::Linux => format!("pkl-linux-{arch}"),
        HostOS::MacOS => format!("pkl-macos-{arch}"),
        HostOS::Windows => format!("pkl-windows-{arch}.exe"),
        _ => unreachable!(),
    };

    Ok(Json(DownloadPrebuiltOutput {
        download_url: format!(
            "https://github.com/apple/pkl/releases/download/{version}/{download_file}"
        ),
        download_name: Some(download_file),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let mut output = LocateExecutablesOutput::default();

    // Releases are standalone executables, not archives, so the downloaded
    // file is renamed to the plugin ID, and not just "pkl"
    let id = get_plugin_id()?;

    output.exes.insert(
        "pkl".into(),
        ExecutableConfig::new_primary(env.os.get_exe_name(&id)),
    );

    Ok(Json(output))
}
