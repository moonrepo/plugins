use crate::config::{PythonPackageManager, PythonToolchainConfig};
use extism_pdk::*;
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use starbase_utils::json::JsonValue;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: "Python".into(),
        description: Some(
            "Installs dependencies and provides sync operations that keep project's in a healthy state."
                .into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        // For project detection
        config_file_globs: vec![
            "poetry.*".into(),
            "uv.*".into(),
            ".python-version".into(),
            ".venv".into(),
        ],
        manifest_file_names: vec![
            "pyproject.toml".into(),
            "requirements.txt".into(),
            // pip
            "Pipfile".into(),
            // poetry
            "poetry.toml".into(),
            // uv
            "uv.toml".into(),
        ],
        lock_file_names: vec![
            ".pylock.toml".into(),
            // pip
            "Pipfile.lock".into(),
            // poetry
            "poetry.lock".into(),
            // uv
            "uv.lock".into(),
        ],
        // For task detection
        exe_names: vec![
            "python".into(),
            "python3".into(),
            "python-3".into(),
            // pip
            "pip".into(),
            // poetry
            "poetry".into(),
            // uv
            "uv".into(),
        ],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(input): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    let mut output = InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/guides/python".into()),
        docs_url: Some("https://moonrepo.dev/docs/config/toolchain#python".into()),
        ..Default::default()
    };

    if let Some(package_manager) = detect_package_manager(&input.context.working_dir)? {
        output.default_settings.insert(
            "packageManager".into(),
            JsonValue::String(package_manager.to_string()),
        );
    } else {
        output.prompts.push(SettingPrompt::new(
            "packageManager",
            "Package manager to install dependencies with?",
            PromptType::Select {
                default_index: 0,
                options: vec![
                    JsonValue::String("pip".into()),
                    JsonValue::String("poetry".into()),
                    JsonValue::String("uv".into()),
                ],
            },
        ));
    }

    Ok(Json(output))
}

fn detect_package_manager(root: &VirtualPath) -> AnyResult<Option<PythonPackageManager>> {
    if root.join("uv.toml").exists() || root.join("uv.lock").exists() {
        return Ok(Some(PythonPackageManager::Uv));
    } else if root.join("poetry.toml").exists() || root.join("poetry.lock").exists() {
        return Ok(Some(PythonPackageManager::Poetry));
    } else if root.join("Pipfile").exists() || root.join("Pipfile.lock").exists() {
        return Ok(Some(PythonPackageManager::Pip));
    }

    Ok(None)
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<PythonToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    let config = parse_toolchain_config::<PythonToolchainConfig>(input.toolchain_config)?;

    Ok(Json(DefineDockerMetadataOutput {
        default_image: Some(format!(
            "python:{}",
            config
                .version
                .as_ref()
                .map(|version| version.to_partial_string())
                .unwrap_or_else(|| "latest".into())
        )),
        scaffold_globs: vec![],
    }))
}
