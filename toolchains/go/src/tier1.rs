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
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/config/toolchain#go".into()),
        docs_url: None,
        prompts: vec![
            SettingPrompt::new(
                "workspaces",
                "Support Go workspaces via <file>go.work</file>?",
                PromptType::Confirm { default: true },
            ),
            SettingPrompt::new(
                "tidyOnChange",
                "Run tidy on dependencies change?",
                PromptType::Confirm { default: false },
            ),
        ],
        ..Default::default()
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
    let config = parse_toolchain_config::<GoToolchainConfig>(input.toolchain_config)?;
    let mut output = PruneDockerOutput::default();
    let vendor_dir = input
        .root
        .join(config.vendor_dir.as_deref().unwrap_or("vendor"));

    if vendor_dir.exists() && input.docker_config.delete_vendor_directories {
        fs::remove_dir_all(&vendor_dir)?;

        if let Some(file) = vendor_dir.virtual_path() {
            output.changed_files.push(file);
        }
    }

    Ok(Json(output))
}
