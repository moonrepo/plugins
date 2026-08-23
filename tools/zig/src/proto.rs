use crate::config::ZigToolConfig;
use crate::releases::ZigReleaseIndex;
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "Zig";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 61, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Toggle(true),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<ZigToolConfig>(),
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
            .map(|version| UnresolvedVersionSpec::parse(format!(">={version}")))
            .transpose()?
    } else {
        input
            .content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(UnresolvedVersionSpec::parse)
            .transpose()?
    };

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let config = get_tool_config::<ZigToolConfig>()?;
    let index: ZigReleaseIndex = fetch_json(&config.index_url)?;
    let canary = index
        .master()
        .map(|release| UnresolvedVersionSpec::parse(&release.version))
        .transpose()?;
    let mut output = LoadVersionsOutput::from(index.stable_versions())?;

    if let Some(canary) = canary {
        output.canary = Some(canary.clone());
        output.aliases.insert("master".into(), canary);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let config = get_tool_config::<ZigToolConfig>()?;
    let index: ZigReleaseIndex = fetch_json(&config.index_url)?;
    let release = index.find(&input.context.version)?;
    let artifact = release.artifact(env)?;
    let filename = artifact
        .tarball
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| plugin_err!("Invalid Zig download URL <url>{}</url>.", artifact.tarball))?;
    let archive_prefix = filename
        .strip_suffix(".tar.xz")
        .or_else(|| filename.strip_suffix(".zip"))
        .ok_or_else(|| plugin_err!("Unsupported Zig archive <file>{filename}</file>."))?;

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(archive_prefix.into()),
        checksum: Some(Checksum::sha256(artifact.shasum)),
        download_name: Some(filename.into()),
        download_url: artifact.tarball,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "zig".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name("zig")),
        )]),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimum_zig_version() {
        assert_eq!(
            parse_zon_version(
                r#".{
                    .name = .example,
                    .minimum_zig_version = "0.14.1", // Required for APIs.
                }"#,
            ),
            Some("0.14.1")
        );
    }

    #[test]
    fn ignores_other_zon_fields() {
        assert_eq!(
            parse_zon_version(
                r#".{
                    .version = "1.2.3",
                }"#,
            ),
            None
        );
    }
}
