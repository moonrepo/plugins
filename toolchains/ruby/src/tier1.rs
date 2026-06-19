use crate::config::RubyToolchainConfig;
use extism_pdk::*;
use moon_config::LanguageType;
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: "Ruby".into(),
        description: Some(
            "Installs Ruby and gems via Bundler, and keeps projects in a healthy state.".into(),
        ),
        language: Some(LanguageType::Ruby),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        // For project detection
        config_file_globs: vec![
            ".ruby-version".into(),
            "*.gemspec".into(),
            ".rubocop.yml".into(),
        ],
        manifest_file_names: vec![
            "Gemfile".into(),
            // Bundler's alternative manifest name
            "gems.rb".into(),
        ],
        lock_file_names: vec![
            "Gemfile.lock".into(),
            // Bundler's alternative lockfile name
            "gems.locked".into(),
        ],
        // For task detection
        exe_names: vec![
            "ruby".into(),
            "bundle".into(),
            "bundler".into(),
            "gem".into(),
            "rake".into(),
            "irb".into(),
        ],
        // `bundle_path` is configurable, but registration does not receive
        // toolchain config, so reporting a static vendor dir would go stale.
        vendor_dir_name: None,
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(input): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    let mut output = InitializeToolchainOutput::default();

    // Bundler is the only dependency manager, so there's nothing to prompt for.
    // As a convenience, pre-fill the version from an existing `.ruby-version`
    // pin (the de facto standard, also read by proto for version detection).
    let version_file = input.context.working_dir.join(".ruby-version");

    if version_file.exists()
        && let Ok(contents) = fs::read_file(&version_file)
    {
        // `.ruby-version` may contain `3.3.5` or rbenv's `ruby-3.3.5` form.
        let version = contents.strip_prefix("ruby-").unwrap_or(&contents).trim();

        if !version.is_empty() {
            output
                .default_settings
                .insert("version".into(), serde_json::Value::String(version.into()));
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<RubyToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    let config = parse_toolchain_config::<RubyToolchainConfig>(input.toolchain_config)?;

    Ok(Json(DefineDockerMetadataOutput {
        default_image: Some(format!(
            "ruby:{}",
            config
                .version
                .as_ref()
                .map(|version| version.to_partial_string())
                .unwrap_or_else(|| "latest".into())
        )),
        scaffold_globs: vec![
            "Gemfile".into(),
            "Gemfile.lock".into(),
            "*.gemspec".into(),
            ".ruby-version".into(),
        ],
    }))
}
