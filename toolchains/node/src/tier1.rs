use crate::config::NodeToolchainConfig;
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
        name: "Node.js".into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_file_globs: vec![".nvmrc".into(), ".node-version".into()],
        exe_names: vec!["node".into(), "nodejs".into()],
        manifest_file_names: vec!["package.json".into()],
        vendor_dir_name: Some("node_modules".into()),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/guides/javascript/node-handbook".into()),
        docs_url: Some("https://moonrepo.dev/docs/config/toolchain#node".into()),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<NodeToolchainConfig>(),
    }))
}

// Node.js images tag `lts`, and each LTS line by its codename, like `jod`
fn get_alias_tag(alias: &str) -> Option<String> {
    match alias {
        "lts" | "lts-latest" | "stable" => Some("lts".into()),
        "canary" | "latest" | "node" => None,
        _ => Some(
            alias
                .strip_prefix("lts-")
                .or_else(|| alias.strip_prefix("lts/"))
                .unwrap_or(alias)
                .into(),
        ),
    }
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    let config = parse_toolchain_config::<NodeToolchainConfig>(input.toolchain_config)?;

    let tag = match &config.version {
        // Images are for production, and `latest` follows the Current release
        None => "lts".into(),
        // Pre-releases aren't published as images
        Some(UnresolvedVersionSpec::Version(version)) if version.prerelease.is_some() => {
            "latest".into()
        }
        Some(UnresolvedVersionSpec::Requirement(req)) if req.prerelease.is_some() => {
            "latest".into()
        }
        version => get_docker_image_tag(version.as_ref(), get_alias_tag),
    };

    Ok(Json(DefineDockerMetadataOutput {
        default_image: Some(format!("node:{tag}")),
        scaffold_globs: vec![
            // postinstall scripts, etc
            "*.{js,cjs,mjs}".into(),
        ],
    }))
}
