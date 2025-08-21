use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;
use toml::Value;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: "uv".into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["self upgrade".into()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec!["uv.toml".into(), "pyproject.toml".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    // https://peps.python.org/pep-0440/#version-specifiers
    fn parse_pep(version: &str) -> AnyResult<UnresolvedVersionSpec> {
        Ok(UnresolvedVersionSpec::parse(
            version
                .replace("~=", "~")
                .replace("===", "^")
                .replace("==", "="),
        )?)
    }

    if input.file == "uv.toml" {
        if let Ok(uv_toml) = toml::from_str::<Value>(&input.content)
            && let Some(Value::String(constraint)) = uv_toml.get("required-version")
        {
            version = Some(parse_pep(constraint)?);
        }
    } else if input.file == "pyproject.toml"
        && let Ok(project_toml) = toml::from_str::<Value>(&input.content)
        && let Some(tool_field) = project_toml.get("tool")
        && let Some(uv_field) = tool_field.get("uv")
        && let Some(Value::String(constraint)) = uv_field.get("required-version")
    {
        version = Some(parse_pep(constraint)?);
    }

    Ok(Json(ParseVersionFileOutput { version }))
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
