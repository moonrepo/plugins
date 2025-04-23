use crate::cargo_toml::CargoToml;
use crate::config::RustToolchainConfig;
use crate::toolchain_toml::ToolchainToml as RustToolchainToml;
use cargo_toml::DepsSet;
use extism_pdk::*;
use moon_config::{DependencyScope, PartialDependencyConfig};
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;
use rust_tool::{ToolchainSection, ToolchainToml};
use schematic::SchemaBuilder;
use starbase_utils::{fs, toml};

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: "Rust".into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_file_globs: vec![
            ".cargo/*.toml".into(),
            "rust-toolchain".into(),
            "rust-toolchain.toml".into(),
        ],
        lock_file_name: Some("Cargo.lock".into()),
        manifest_file_name: Some("Cargo.toml".into()),
        exe_names: vec![
            "cargo".into(),
            "rustc".into(),
            "rustdoc".into(),
            "rustfmt".into(),
            "rustup".into(),
        ],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<RustToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/guides/rust/handbook".into()),
        docs_url: Some("https://moonrepo.dev/docs/config/toolchain#rust".into()),
        prompts: vec![],
        ..Default::default()
    }))
}

// TODO
#[plugin_fn]
pub fn extend_project(
    Json(input): Json<ExtendProjectInput>,
) -> FnResult<Json<ExtendProjectOutput>> {
    let mut output = ExtendProjectOutput::default();
    let cargo_toml_path = input
        .context
        .workspace_root
        .join(&input.project.source)
        .join("Cargo.toml");

    let mut extract_implicit_deps = |package_deps: &DepsSet, scope: DependencyScope| {
        for (dep_name, dep) in package_deps {
            // Only inherit if the dependency is using the local `path = "..."` syntax
            if dep.detail().is_some_and(|d| d.path.is_some()) {
                output.dependencies.insert(
                    dep_name.to_owned(),
                    PartialDependencyConfig {
                        scope: Some(scope),
                        via: Some(dep_name.to_owned()),
                        ..Default::default()
                    },
                );
            }
        }
    };

    if cargo_toml_path.exists() {
        let cargo = CargoToml::load(cargo_toml_path)?;

        if let Some(package) = &cargo.package {
            output.alias = Some(package.name.clone());

            extract_implicit_deps(&cargo.dependencies, DependencyScope::Production);
            extract_implicit_deps(&cargo.dev_dependencies, DependencyScope::Development);
            extract_implicit_deps(&cargo.build_dependencies, DependencyScope::Build);
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn setup_toolchain(
    Json(input): Json<SetupToolchainInput>,
) -> FnResult<Json<SetupToolchainOutput>> {
    let mut output = SetupToolchainOutput::default();
    let config = parse_toolchain_config::<RustToolchainConfig>(input.toolchain_config)?;
    let cargo_root = input.context.workspace_root.join(&config.root);

    if cargo_root.join("Cargo.lock").exists() && config.sync_toolchain_config {
        let legacy_toolchain_path = cargo_root.join("rust-toolchain");
        let toolchain_path = cargo_root.join("rust-toolchain.toml");

        // Convert rust-toolchain to rust-toolchain.toml
        if legacy_toolchain_path.exists() {
            let legacy_contents = fs::read_file(&legacy_toolchain_path)?;

            if legacy_contents.contains("[toolchain]") {
                fs::rename(&legacy_toolchain_path, &toolchain_path)?;
            } else {
                fs::remove_file(&legacy_toolchain_path)?;

                toml::write_file(
                    &toolchain_path,
                    &ToolchainToml {
                        toolchain: ToolchainSection {
                            channel: Some(legacy_contents),
                        },
                    },
                    true,
                )?;
            }

            output.changed_files.push(legacy_toolchain_path);
            output.changed_files.push(toolchain_path.clone());
        }

        let mut toolchain_toml = RustToolchainToml::load(toolchain_path)?;
        toolchain_toml.data.toolchain.channel = Some(input.configured_version.to_string());

        if let Some(file) = toolchain_toml.save()? {
            output.changed_files.push(file);
        }
    }

    Ok(Json(output))
}
