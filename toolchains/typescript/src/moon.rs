use crate::config::TypeScriptConfig;
use crate::run_task::*;
use crate::sync_project::*;
use crate::tsconfig_json::TsConfigJson;
use extism_pdk::*;
use moon_pdk::{is_project_toolchain_enabled, parse_toolchain_config};
use moon_pdk_api::*;
use schematic::SchemaBuilder;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: "TypeScript".into(),
        description: Some(
            "Provides sync operations that keep <file>tsconfig.json</file>'s in a healthy state."
                .into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_file_globs: vec![
            "tsconfig.json".into(),
            "tsconfig.*.json".into(),
            "*.tsconfig.json".into(),
            ".tsbuildinfo".into(),
            "*.tsbuildinfo".into(),
        ],
        exe_names: vec!["tsc".into(), "tsserver".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<TypeScriptConfig>(),
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/config/toolchain#typescript".into()),
        docs_url: Some(
            "https://moonrepo.dev/docs/guides/javascript/typescript-project-refs".into(),
        ),
        prompts: vec![
            {
                let mut prompt = SettingPrompt::new(
                    "syncProjectReferences",
                    "Sync project references?",
                    PromptType::Confirm { default: true },
                );
                prompt.prompts.extend([
                    SettingPrompt::new_full(
                        "syncProjectReferencesToPaths",
                        "Sync project references as <property>paths</property> aliases?",
                        PromptType::Confirm { default: false },
                    ),
                    SettingPrompt::new_full(
                        "includeProjectReferenceSources",
                        "Append sources of project reference to each project's <property>include</property>?",
                        PromptType::Confirm { default: false },
                    ),
                ]);
                prompt
            },
            SettingPrompt::new(
                "includeSharedTypes",
                "Append shared types to each project's <property>include</property>?",
                PromptType::Confirm { default: false },
            ),
        ],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn sync_project(Json(input): Json<SyncProjectInput>) -> FnResult<Json<SyncOutput>> {
    let mut output = SyncOutput::default();

    if is_project_toolchain_enabled(&input.project) {
        let config = parse_toolchain_config::<TypeScriptConfig>(input.toolchain_config)?;

        output.changed_files = sync_project_references(
            &input.context,
            &config,
            &input.project,
            &input.project_dependencies,
        )?;
    } else {
        output.skipped = true;
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn hash_task_contents(
    Json(input): Json<HashTaskContentsInput>,
) -> FnResult<Json<HashTaskContentsOutput>> {
    let config = parse_toolchain_config::<TypeScriptConfig>(input.toolchain_config)?;
    let mut output = HashTaskContentsOutput::default();
    let mut data = json::json!({});
    let mut has_data = false;

    for tsconfig_path in [
        input
            .context
            .workspace_root
            .join(&config.root)
            .join(&config.root_config_file_name),
        input
            .context
            .workspace_root
            .join(&config.root)
            .join(&config.root_options_config_file_name),
        input
            .context
            .workspace_root
            .join(&input.project.source)
            .join(&config.project_config_file_name),
    ] {
        if tsconfig_path.exists() {
            let tsconfig = TsConfigJson::load_with_extends(tsconfig_path)?;

            if let Some(options) = &tsconfig.compiler_options {
                let next_data = hash_compiler_options(options);

                data = starbase_utils::json::merge(&data, &next_data);
                has_data = true;
            }
        }
    }

    if has_data && data.as_object().is_some_and(|obj| !obj.is_empty()) {
        output.contents.push(data);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    let config = parse_toolchain_config::<TypeScriptConfig>(input.toolchain_config)?;
    let mut output = DefineDockerMetadataOutput::default();

    let with_root = |name: String| {
        if config.root.is_empty() || config.root == "." {
            name
        } else {
            format!("{}/{name}", config.root)
        }
    };

    output.scaffold_globs.push(config.project_config_file_name);
    output
        .scaffold_globs
        .push(with_root(config.root_config_file_name));
    output
        .scaffold_globs
        .push(with_root(config.root_options_config_file_name));

    output.scaffold_globs.extend(vec![
        "tsconfig.json".into(),
        "tsconfig.*.json".into(),
        "*.tsconfig.json".into(),
    ]);

    Ok(Json(output))
}
