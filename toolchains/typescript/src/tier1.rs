use crate::config::TypeScriptToolchainConfig;
use crate::context::*;
use crate::tier1_sync::*;
use extism_pdk::*;
use moon_config::LanguageType;
use moon_pdk::{is_project_toolchain_enabled, parse_toolchain_config};
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: "TypeScript".into(),
        description: Some(
            "Provides sync operations that keep <file>tsconfig.json</file>'s in a healthy state."
                .into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        language: Some(LanguageType::TypeScript),
        config_file_globs: vec![
            "tsconfig.json".into(),
            "tsconfig.*.json".into(),
            "*.tsconfig.json".into(),
            ".tsbuildinfo".into(),
            "*.tsbuildinfo".into(),
        ],
        // This intercepts other toolchains from being inherited,
        // like node and javascript, which are far more important!
        // Once plugins are stabilized, we can require javascript
        // and avoid some of this headache.
        // exe_names: vec!["tsc".into(), "tsserver".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<TypeScriptToolchainConfig>(),
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
        let config = parse_toolchain_config::<TypeScriptToolchainConfig>(input.toolchain_config)?;
        let context = create_typescript_context(&input.context, &config, &input.project);

        let (op, files) = Operation::track("sync-project-references", || {
            sync_project_references(
                &context,
                &config,
                &input.project,
                &input.project_dependencies,
            )
        })?;

        output.operations.push(op);
        output
            .changed_files
            .extend(files.into_iter().filter_map(|file| file.virtual_path()));
    } else {
        output.skipped = true;
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    let config = parse_toolchain_config::<TypeScriptToolchainConfig>(input.toolchain_config)?;
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
        // postinstall scripts, etc
        "*.{ts,tsx,cts,mts}".into(),
    ]);

    Ok(Json(output))
}
