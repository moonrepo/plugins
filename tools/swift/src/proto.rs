use crate::config::SwiftToolConfig;
use crate::version::{from_swift_version, to_swift_version};
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "Swift";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<SwiftToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".swift-version".into(), "Package.swift".into()],
        ignore: vec![".build".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == ".swift-version" {
        if let Some(line) = input.content.lines().find(|line| !line.trim().is_empty()) {
            version = Some(UnresolvedVersionSpec::parse(line.trim())?);
        }
    } else if input.file == "Package.swift" {
        for line in input.content.lines() {
            let line = line.trim();

            if let Some(raw_version) = line.strip_prefix("// swift-tools-version:") {
                let raw_version = raw_version.trim();
                let range = format!("^{raw_version}");

                version = Some(UnresolvedVersionSpec::parse(range)?);
                break;
            }
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/swiftlang/swift")?
        .into_iter()
        .filter_map(|tag| {
            tag.strip_prefix("swift-")
                .and_then(|tag| tag.strip_suffix("-RELEASE"))
                .map(from_swift_version)
        })
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let version = &input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: NAME.into()
        }));
    }

    check_supported_os_and_arch(
        NAME,
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64],
        ],
    )?;

    if matches!(env.libc, HostLibc::Musl) {
        return Err(plugin_err!(
            "No pre-built Swift archive is available for musl Linux targets.",
        ));
    }

    let config = get_tool_config::<SwiftToolConfig>()?;
    let version = to_swift_version(version);
    let release = format!("swift-{version}-release");
    let folder = format!("swift-{version}-RELEASE");

    let (platform, archive_suffix) = match env.arch {
        HostArch::Arm64 => (
            format!("{}-aarch64", config.linux_platform),
            format!("{}-aarch64", config.linux_archive_suffix),
        ),
        HostArch::X64 => (config.linux_platform, config.linux_archive_suffix),
        _ => unreachable!(),
    };

    let archive_prefix = format!("{folder}-{archive_suffix}");
    let filename = format!("{archive_prefix}.tar.gz");
    let download_url = config
        .dist_url
        .replace("{release}", &release)
        .replace("{platform}", &platform)
        .replace("{folder}", &folder)
        .replace("{file}", &filename);

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(archive_prefix),
        download_name: Some(filename),
        download_url,
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
                "swift".into(),
                ExecutableConfig::new_primary(env.os.get_exe_name("usr/bin/swift")),
            ),
            (
                "swiftc".into(),
                ExecutableConfig::new(env.os.get_exe_name("usr/bin/swiftc")),
            ),
            (
                "sourcekit-lsp".into(),
                ExecutableConfig::new(env.os.get_exe_name("usr/bin/sourcekit-lsp")),
            ),
        ]),
        exes_dirs: vec!["usr/bin".into()],
        globals_lookup_dirs: vec!["$TOOL_DIR/usr/bin".into()],
        ..Default::default()
    }))
}
