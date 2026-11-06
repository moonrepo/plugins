use crate::config::GoToolchainConfig;
use crate::go_mod::{Replacement, parse_go_mod};
use crate::go_work::GoWork;
use crate::package_graph::resolve_source_path;
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
        name: "Go".into(),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        language: Some(LanguageType::Go),
        config_file_globs: vec![],
        exe_names: vec!["go".into(), "gofmt".into()],
        lock_file_names: vec!["go.sum".into(), "go.work.sum".into()],
        manifest_file_names: vec!["go.mod".into(), "go.work".into()],
        // Modules are installed into the global module cache, while
        // `vendor` is committed source, so there's no vendor directory
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
        default_image: Some(get_docker_image(config.version.as_ref())),
        ..Default::default()
    }))
}

// Image tags are plain versions (`1.24`, `1.24.3`), so requirements
// like `~1.24` must drop their operator
fn get_docker_image(version: Option<&UnresolvedVersionSpec>) -> String {
    match version {
        Some(version)
            if !version.is_canary() && !matches!(version, UnresolvedVersionSpec::Alias(_)) =>
        {
            format!("golang:{}", version.to_partial_string())
        }
        _ => "golang:latest".into(),
    }
}

#[plugin_fn]
pub fn scaffold_docker(
    Json(input): Json<ScaffoldDockerInput>,
) -> FnResult<Json<ScaffoldDockerOutput>> {
    let mut output = ScaffoldDockerOutput::default();

    // `go mod download` loads every workspace member and local `replace`
    // target, but the configs phase only has the manifests of projects
    if input.phase != ScaffoldDockerPhase::Configs {
        return Ok(Json(output));
    }

    let source = input
        .project
        .as_ref()
        .map(|project| project.source.as_str())
        .filter(|source| *source != ".")
        .unwrap_or_default();
    let mut dirs = vec![];

    let go_work_path = input.input_dir.join("go.work");

    if go_work_path.exists() {
        dirs.extend(GoWork::parse(fs::read_file(go_work_path)?)?.modules);
    }

    let go_mod_path = input.input_dir.join("go.mod");

    if go_mod_path.exists() {
        for replacement in parse_go_mod(fs::read_file(go_mod_path)?)?.replace {
            if let Replacement::FilePath(path) = replacement.replacement {
                dirs.push(path);
            }
        }
    }

    // Output paths mirror the workspace, starting from the skeleton root
    let mut output_root = input.output_dir.clone();

    for _ in source.split('/').filter(|part| !part.is_empty()) {
        if let Some(parent) = output_root.parent() {
            output_root = parent;
        }
    }

    for dir in dirs {
        let Some(dir) = resolve_source_path(source, &dir) else {
            continue;
        };

        for file_name in ["go.mod", "go.sum"] {
            let file = input.context.workspace_root.join(&dir).join(file_name);

            if file.exists() {
                let copy = output_root.join(&dir).join(file_name);

                fs::copy_file(&file, &copy)?;
                output.copied_files.push(copy);
            }
        }
    }

    Ok(Json(output))
}
