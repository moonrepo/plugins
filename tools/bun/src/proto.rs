use crate::config::BunPluginConfig;
use extism_pdk::*;
use lang_javascript_common::{
    extract_engine_version, extract_package_manager_version, extract_version_from_text,
    extract_volta_version,
};
use nodejs_package_json::PackageJson;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "Bun";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        config_schema: Some(SchemaBuilder::build_root::<BunPluginConfig>()),
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["upgrade".into()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![
            ".bumrc".into(),
            ".bun-version".into(),
            "package.json".into(),
        ],
        ignore: vec!["node_modules".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == "package.json" {
        if let Ok(package_json) = json::from_str::<PackageJson>(&input.content) {
            if let Some(constraint) = extract_volta_version(&package_json, &input.path, "bun")? {
                version = Some(UnresolvedVersionSpec::parse(constraint)?);
            }

            if version.is_none() {
                if let Some(constraint) = extract_engine_version(&package_json, "bun") {
                    version = Some(UnresolvedVersionSpec::parse(constraint)?);
                }
            }

            if version.is_none() {
                if let Some(constraint) = extract_package_manager_version(&package_json, "bun") {
                    version = Some(UnresolvedVersionSpec::parse(constraint)?);
                }
            }
        }
    } else if let Some(constraint) = extract_version_from_text(&input.content) {
        version = Some(UnresolvedVersionSpec::parse(constraint)?);
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/oven-sh/bun")?
        .into_iter()
        .filter_map(|tag| tag.strip_prefix("bun-v").map(|tag| tag.to_owned()))
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let has_windows_support = match &input.context.version {
        VersionSpec::Canary => true,
        VersionSpec::Alias(alias) => alias == "latest",
        VersionSpec::Semantic(version) => version.major >= 1 && version.minor >= 1,
        _ => false,
    };

    check_supported_os_and_arch(
        NAME,
        &env,
        if has_windows_support {
            permutations! [
                HostOS::Linux => [HostArch::X64, HostArch::Arm64],
                HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
                HostOS::Windows => [HostArch::X64],
            ]
        } else {
            permutations! [
                HostOS::Linux => [HostArch::X64, HostArch::Arm64],
                HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            ]
        },
    )?;

    let version = &input.context.version;

    let arch = match env.arch {
        HostArch::Arm64 => "aarch64",
        HostArch::X64 => "x64",
        _ => unreachable!(),
    };

    let mut avx2_suffix = "";

    if env.arch == HostArch::X64 && env.os.is_linux() && command_exists(&env, "grep") {
        let output = exec_captured("grep", ["avx2", "/proc/cpuinfo"])?;

        if output.exit_code != 0 {
            avx2_suffix = "-baseline";
        }
    };

    let prefix = match env.os {
        HostOS::Linux => format!("bun-linux-{arch}{avx2_suffix}"),
        HostOS::MacOS => format!("bun-darwin-{arch}{avx2_suffix}"),
        HostOS::Windows => format!("bun-windows-{arch}"),
        _ => unreachable!(),
    };

    let filename = format!("{prefix}.zip");
    let mut host = get_tool_config::<BunPluginConfig>()?.dist_url;

    // canary - bun-v1.2.3
    if version.is_canary() {
        host = host.replace("bun-v{version}", "{version}");
    };

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(prefix),
        download_url: host
            .replace("{version}", &version.to_string())
            .replace("{file}", &filename),
        download_name: Some(filename),
        // Checksums are not consistently updated
        checksum_url: if version.is_canary() {
            None
        } else {
            Some(
                host.replace("{version}", &version.to_string())
                    .replace("{file}", "SHASUMS256.txt"),
            )
        },
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    let bunx = ExecutableConfig {
        // `bunx` isn't a real binary provided by Bun so we can't symlink it.
        // Instead, it's simply the `bun` binary named `bunx` and Bun toggles
        // functionality based on `args[0]`.
        exe_link_path: Some(env.os.get_exe_name("bun").into()),

        // The approach doesn't work for shims since we use child processes,
        // so execute `bun x` instead (notice the space).
        shim_before_args: Some(StringOrVec::String("x".into())),

        ..ExecutableConfig::default()
    };

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                "bun".into(),
                ExecutableConfig::new_primary(env.os.get_exe_name("bun")),
            ),
            ("bunx".into(), bunx),
        ]),
        globals_lookup_dirs: vec!["$HOME/.bun/bin".into()],
        ..LocateExecutablesOutput::default()
    }))
}
