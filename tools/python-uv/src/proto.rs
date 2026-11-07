use extism_pdk::*;
use lang_python_common::parse_pep440_spec;
use proto_pdk::*;
use starbase_utils::fs;
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
        minimum_proto_version: Some(Version::new(0, 60, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["self update".into()],
        ..Default::default()
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
    let content = fs::read_file(&input.path)?;
    let mut version = None;

    if input.file == "uv.toml" {
        if let Ok(uv_toml) = toml::from_str::<Value>(&content)
            && let Some(Value::String(constraint)) = uv_toml.get("required-version")
        {
            version = Some(parse_pep440_spec(constraint)?);
        }
    } else if input.file == "pyproject.toml"
        && let Ok(project_toml) = toml::from_str::<Value>(&content)
        && let Some(tool_field) = project_toml.get("tool")
        && let Some(uv_field) = tool_field.get("uv")
        && let Some(Value::String(constraint)) = uv_field.get("required-version")
    {
        version = Some(parse_pep440_spec(constraint)?);
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
        env,
        permutations! [
            HostOS::Linux => [
                HostArch::X64, HostArch::Arm64, HostArch::X86, HostArch::Arm,
                HostArch::Powerpc64, HostArch::S390x, HostArch::Riscv64
            ],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64, HostArch::Arm64, HostArch::X86],
        ],
    )?;

    let version = input.context.version;
    let arch = match env.arch {
        HostArch::X86 => "i686".into(),
        HostArch::Powerpc64 => "powerpc64le".into(), // Little endian, like most Linux hosts
        HostArch::Riscv64 => "riscv64gc".into(),
        arch => arch.to_rust_arch(),
    };

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: "uv".into()
        }));
    }

    let target = match env.os {
        // The ARMv6 build is static, so it runs on ARMv7 and any libc
        HostOS::Linux if env.arch == HostArch::Arm => format!("{arch}-unknown-linux-musleabihf"),
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
        // Windows zips have no top-level folder
        archive_prefix: (!env.os.is_windows()).then_some(target_name),
        checksum_url: Some(format!("{base_url}/{checksum_file}")),
        checksum_name: Some(checksum_file),
        download_url: format!("{base_url}/{download_file}"),
        download_name: Some(download_file),
        ..Default::default()
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
        ..Default::default()
    }))
}
