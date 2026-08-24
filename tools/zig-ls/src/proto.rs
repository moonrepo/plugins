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

    let archive_prefix = filename
        .strip_suffix(".tar.xz")
        .or_else(|| filename.strip_suffix(".zip"))
        .ok_or_else(|| plugin_err!("Unsupported ZLS archive <file>{filename}</file>."))?;

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
            "zls".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name("zls")),
        )]),
        ..Default::default()
    }))
}
