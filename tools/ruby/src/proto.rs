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
        name: "Ruby".into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 42, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Message(
            "Pre-builds are provided by ruby/ruby-builder, which may not support all versions. Windows is currently not supported."
                .into(),
        ),
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".ruby-version".into()],
        ignore: vec!["vendor".into()],
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/ruby/ruby")?
        .into_iter()
        .filter_map(|tag| {
            if let Some(tag) = tag.strip_prefix('v') {
                // First 2 underscores are the separators between the major,
                // minor, and patch digits, while the remaining underscores
                // are used in the pre/build metadata
                let version = tag.replacen('_', ".", 2).replace('_', "-");

                // Very old versions that we don't need to support
                if version.starts_with('0') || version.starts_with('1') {
                    None
                } else {
                    Some(version)
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        "Ruby",
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            // HostOS::Windows => [HostArch::X64],
        ],
    )?;

    let version = input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: "Ruby".into()
        }));
    }

    let target = match env.os {
        HostOS::Linux => format!("ruby-{version}-ubuntu-20.04"),
        HostOS::MacOS => match env.arch {
            HostArch::X64 => format!("ruby-{version}-macos-latest"),
            HostArch::Arm64 => format!("ruby-{version}-macos-13-arm64"),
            _ => unreachable!(),
        },
        // HostOS::Windows => format!("{arch}-pc-windows-msvc"),
        _ => unreachable!(),
    };

    let download_file = format!("{target}.tar.gz");
    let base_url = "https://github.com/ruby/ruby-builder/releases/download/toolcache";

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: match env.arch {
            HostArch::X64 => Some("x64".into()),
            HostArch::Arm64 => Some("arm64".into()),
            _ => None,
        },
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
                "ruby".into(),
                ExecutableConfig::new_primary(env.os.get_exe_name("bin/ruby")),
            ),
            (
                "rake".into(),
                ExecutableConfig::new(env.os.get_exe_name("bin/rake")),
            ),
            (
                "gem".into(),
                ExecutableConfig::new(env.os.get_exe_name("bin/gem")),
            ),
            (
                "bundle".into(),
                ExecutableConfig::new(env.os.get_exe_name("bin/bundle")),
            ),
        ]),
        exes_dir: Some("bin".into()),
        globals_lookup_dirs: vec![],
        ..LocateExecutablesOutput::default()
    }))
}
