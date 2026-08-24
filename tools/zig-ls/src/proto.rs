use crate::config::ZlsToolConfig;
use crate::releases::ZlsReleaseIndex;
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "ZLS";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 61, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Toggle(true),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<ZlsToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![
            ".zig-version".into(),
            ".zigversion".into(),
            "build.zig.zon".into(),
        ],
        ignore: vec![".zig-cache".into(), "zig-out".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let version = if input.file == "build.zig.zon" {
        parse_zon_version(&input.content)
            .map(|version| to_zls_version(version, ZlsRequirement::MinimumMinor))
            .transpose()?
    } else {
        input
            .content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(|version| to_zls_version(version, ZlsRequirement::CompatibleMinor))
            .transpose()?
    };

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let config = get_tool_config::<ZlsToolConfig>()?;
    let index: ZlsReleaseIndex = fetch_json(&config.index_url)?;

    Ok(Json(LoadVersionsOutput::from(index.stable_versions())?))
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

    let config = get_tool_config::<ZlsToolConfig>()?;
    let index: ZlsReleaseIndex = fetch_json(&config.index_url)?;
    let release = index.find(version)?;
    let artifact = release.artifact(env)?;

    let filename = artifact
        .tarball
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| plugin_err!("Invalid ZLS download URL <url>{}</url>.", artifact.tarball))?;

    let archive_prefix = [".tar.xz", ".tar.gz", ".zip"]
        .into_iter()
        .find_map(|suffix| filename.strip_suffix(suffix))
        .ok_or_else(|| plugin_err!("Unsupported ZLS archive <file>{filename}</file>."))?;

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(archive_prefix.into()),
        checksum: Some(Checksum::sha256(artifact.shasum)),
        download_name: Some(filename.into()),
        download_url: artifact.tarball,
        ..Default::default()
    }))
}

fn parse_zon_version(content: &str) -> Option<&str> {
    content.lines().find_map(|line| {
        let line = line.split_once("//").map_or(line, |(code, _)| code);
        let (key, value) = line.split_once('=')?;

        if key.trim() != ".minimum_zig_version" {
            return None;
        }

        let version = value.trim().trim_end_matches(',').trim();
        version.strip_prefix('"')?.strip_suffix('"')
    })
}

#[derive(Clone, Copy)]
enum ZlsRequirement {
    CompatibleMinor,
    MinimumMinor,
}

fn to_zls_version(
    zig_version: &str,
    requirement: ZlsRequirement,
) -> AnyResult<UnresolvedVersionSpec> {
    if matches!(zig_version, "canary" | "master") {
        return Ok(UnresolvedVersionSpec::parse("canary")?);
    }

    let version = Version::parse(zig_version)?;

    if version.prerelease.is_some() {
        Ok(UnresolvedVersionSpec::parse("canary")?)
    } else {
        let operator = match requirement {
            ZlsRequirement::CompatibleMinor => "~",
            ZlsRequirement::MinimumMinor => ">=",
        };

        Ok(UnresolvedVersionSpec::parse(format!(
            "{operator}{}.{}",
            version.major, version.minor,
        ))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimum_zig_version() {
        assert_eq!(
            parse_zon_version(
                r#".{
                    .name = .example,
                    .minimum_zig_version = "0.15.2", // Required for APIs.
                }"#,
            ),
            Some("0.15.2")
        );
    }

    #[test]
    fn converts_stable_zig_versions_to_zls_minor_ranges() {
        assert_eq!(
            to_zls_version("0.15.2", ZlsRequirement::CompatibleMinor)
                .unwrap()
                .to_string(),
            "~0.15"
        );
        assert_eq!(
            to_zls_version("0.15.2", ZlsRequirement::MinimumMinor)
                .unwrap()
                .to_string(),
            ">=0.15"
        );
    }

    #[test]
    fn converts_development_zig_versions_to_canary() {
        assert_eq!(
            to_zls_version("0.16.0-dev.123+abc", ZlsRequirement::CompatibleMinor)
                .unwrap()
                .to_string(),
            "canary"
        );
        assert_eq!(
            to_zls_version("master", ZlsRequirement::CompatibleMinor)
                .unwrap()
                .to_string(),
            "canary"
        );
    }
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "zls".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name("zls")),
        )]),
        ..Default::default()
    }))
}
