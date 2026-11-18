use crate::config::BunToolchainConfig;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use toolchain_common::{enable_tracing, get_docker_image_tag};

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: "Bun".into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_file_globs: vec![".bumrc".into(), ".bun-version".into(), "bunfig.toml".into()],
        exe_names: vec!["bun".into(), "bunx".into()],
        manifest_file_names: vec!["package.json".into()],
        lock_file_names: vec!["bun.lock".into(), "bun.lockb".into()],
        vendor_dir_name: Some("node_modules".into()),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/guides/javascript/bun-handbook".into()),
        docs_url: Some("https://moonrepo.dev/docs/config/toolchain#bun".into()),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<BunToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    let config = parse_toolchain_config::<BunToolchainConfig>(input.toolchain_config)?;

    let tag = get_docker_image_tag(config.version.as_ref(), |alias| {
        (alias == "canary").then(|| alias.into())
    });

    Ok(Json(DefineDockerMetadataOutput {
        default_image: Some(format!("oven/bun:{tag}")),
        scaffold_globs: vec![
            // postinstall scripts, etc
            "*.{js,cjs,mjs}".into(),
        ],
    }))
}
