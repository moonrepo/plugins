use crate::config::GoToolchainConfig;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use starbase_utils::fs;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: "Go".into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_file_globs: vec![],
        exe_names: vec!["go".into(), "gofmt".into()],
        lock_file_names: vec!["go.sum".into(), "go.work.sum".into()],
        manifest_file_names: vec!["go.mod".into(), "go.work".into()],
        vendor_dir_name: Some("vendor".into()), // go mod vendor
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<GoToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    let config = parse_toolchain_config::<GoToolchainConfig>(input.toolchain_config)?;

    Ok(Json(DefineDockerMetadataOutput {
        default_image: Some(format!(
            "golang:{}",
            config
                .version
                .as_ref()
                .map(|version| version.to_string())
                .unwrap_or_else(|| "latest".into())
        )),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let mut output = PruneDockerOutput::default();
    let vendor_dir = input.root.join("vendor");

    if vendor_dir.exists() && input.docker_config.delete_vendor_directories {
        fs::remove_dir_all(&vendor_dir)?;

        if let Some(file) = vendor_dir.virtual_path() {
            output.changed_files.push(file);
        }
    }

    Ok(Json(output))
}
