use crate::config::RubyToolchainConfig;
use extism_pdk::*;
use moon_config::LanguageType;
use moon_pdk::{exec, get_toolchain_config, parse_toolchain_config, parse_toolchain_config_schema};
use moon_pdk_api::*;
use ruby_tool::parse_ruby_version_file;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();
    let config = get_toolchain_config::<RubyToolchainConfig>()?;

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
            ".rspec".into(),
            "Rakefile".into(),
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
            "rails".into(),
            "rspec".into(),
            "rubocop".into(),
        ],
        vendor_dir_name: Some(config.bundle_path),
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
        && let Some(version) = parse_ruby_version_file(&contents)
    {
        output
            .default_settings
            .insert("version".into(), serde_json::Value::String(version.into()));
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
        // Manifests, lockfiles, and config files are included automatically.
        // Gemspecs from `bundle gem` require their version file, so it's
        // needed for `bundle install` in the configs phase.
        scaffold_globs: vec!["lib/**/version.rb".into()],
    }))
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let config = parse_toolchain_config_schema::<RubyToolchainConfig>(input.toolchain_config)?;
    let mut output = PruneDockerOutput::default();

    // Production installs only skip these groups, so persist them,
    // otherwise `bundle exec` fails on the missing gems at runtime
    let groups = config.get_non_production_groups().join(":");

    exec_bundle(
        &config,
        &input.root,
        ["config", "set", "--local", "without", &groups],
    )?;

    output.changed_files.push(input.root.join(".bundle/config"));

    // Removes the gems in skipped groups but keeps the rest,
    // so native extensions aren't compiled again
    let bundle_dir = input.root.join(crate::tier2::get_bundle_path()?);

    if input.docker_config.delete_vendor_directories && bundle_dir.exists() {
        exec_bundle(&config, &input.root, ["clean"])?;

        output.changed_files.push(bundle_dir);
    }

    Ok(Json(output))
}

fn exec_bundle<const N: usize>(
    config: &RubyToolchainConfig,
    root: &VirtualPath,
    args: [&str; N],
) -> AnyResult<()> {
    let mut command = ExecCommandInput::pipe("bundle", args).cwd(root.to_owned());
    config.pin_version(&mut command);

    let result = exec(command)?;

    if result.exit_code != 0 {
        return Err(anyhow!(
            "Failed to run <shell>bundle {}</shell>: {}",
            args.join(" "),
            result.stderr.trim()
        ));
    }

    Ok(())
}
