use crate::config::ZigToolConfig;
use crate::releases::ZigReleaseIndex;
use extism_pdk::*;
use lang_zig_common::{
    create_download_prebuilt_output, detect_zig_version_files, parse_zig_version_file,
};
use proto_pdk::*;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use std::collections::HashMap;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

static NAME: &str = "Zig";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 64, 0)),
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
    Ok(Json(detect_zig_version_files()))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let content = fs::read_file(&input.path)?;
    let version = parse_zig_version_file(&input.file, &content)
        .map(|source| source.to_zig_requirement())
        .transpose()?;

    Ok(Json(ParseVersionFileOutput {
        version,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let config = get_tool_config::<ZigToolConfig>()?;

    // The index is keyed by version, and the canary by master
    let response = load_tool_versions(LoadToolVersionsInput::Index {
        url: config.index_url,
        version_key: Some("*~".into()),
        headers: Default::default(),
        include_body: true,
    })?;
    let mut output = LoadVersionsOutput::from(
        response
            .versions
            .into_iter()
            .filter(|version| version != "master")
            .collect(),
    )?;

    // The body isn't returned for a source configured by the user
    let canary = match response.body {
        // ADR-0008.5: Skip the canary when its version can't be parsed
        Some(body) => json::from_str::<ZigReleaseIndex>(&body)?
            .master()
            .and_then(|release| {
                UnresolvedVersionSpec::parse(&release.version)
                    .inspect_err(|error| debug!("Skipping the canary {}: {error}", release.version))
                    .ok()
            }),
        None => None,
    };

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

    Ok(Json(create_download_prebuilt_output(NAME, artifact)?))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let mut primary = ExecutableConfig::new_primary(env.os.get_exe_name("zig"));

    // Zig looks for `lib` next to its own path, so the copy that
    // proto makes for a Windows bin can't run
    primary.no_bin = env.os.is_windows();

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([("zig".into(), primary)]),
        ..Default::default()
    }))
}
