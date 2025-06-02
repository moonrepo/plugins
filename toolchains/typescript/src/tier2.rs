use crate::config::TypeScriptToolchainConfig;
use crate::context::*;
use crate::tsconfig_json::TsConfigJson;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;
use starbase_utils::json::{JsonMap, JsonValue};
use typescript_tsconfig_json::CompilerOptions;

/// Hash compiler options that may alter compiled/generated output.
fn hash_compiler_options(compiler_options: &CompilerOptions) -> JsonValue {
    let mut options = JsonMap::default();

    if let Some(jsx) = &compiler_options.jsx {
        options.insert("jsx".into(), json::to_value(jsx).unwrap());
    }

    if let Some(jsx_factory) = &compiler_options.jsx_factory {
        options.insert("jsxFactory".into(), jsx_factory.to_string().into());
    }

    if let Some(jsx_fragment_factory) = &compiler_options.jsx_fragment_factory {
        options.insert(
            "jsxFragmentFactory".into(),
            jsx_fragment_factory.to_string().into(),
        );
    }

    if let Some(jsx_import_source) = &compiler_options.jsx_import_source {
        options.insert(
            "jsxImportSource".into(),
            jsx_import_source.to_string().into(),
        );
    }

    if let Some(lib) = &compiler_options.lib {
        options.insert("lib".into(), json::to_value(lib).unwrap());
    }

    if let Some(module) = &compiler_options.module {
        options.insert("module".into(), json::to_value(module).unwrap());
    }

    if let Some(module_resolution) = &compiler_options.module_resolution {
        options.insert(
            "moduleResolution".into(),
            json::to_value(module_resolution).unwrap(),
        );
    }

    if let Some(target) = &compiler_options.target {
        options.insert("target".into(), json::to_value(target).unwrap());
    }

    JsonValue::Object(options)
}

#[plugin_fn]
pub fn hash_task_contents(
    Json(input): Json<HashTaskContentsInput>,
) -> FnResult<Json<HashTaskContentsOutput>> {
    let config = parse_toolchain_config::<TypeScriptToolchainConfig>(input.toolchain_config)?;
    let context = create_typescript_context(&input.context, &config, &input.project);
    let mut output = HashTaskContentsOutput::default();
    let mut data = json::json!({});
    let mut has_data = false;

    for tsconfig_path in [
        context.root_config,
        context.root_options_config,
        context.project_config,
    ] {
        if tsconfig_path.exists() {
            // Don't error if extending fails, as one of the files may not
            // exist yet, as they could be dynamically generated on-demand
            match TsConfigJson::load_with_extends(tsconfig_path.clone()) {
                Ok(tsconfig) => {
                    if let Some(options) = &tsconfig.compiler_options {
                        let next_data = hash_compiler_options(options);

                        data = starbase_utils::json::merge(&data, &next_data);
                        has_data = true;
                    }
                }
                Err(error) => {
                    debug!(
                        "Failed to load extends chain for {}: {error}",
                        tsconfig_path
                    );
                }
            }
        }
    }

    if has_data && data.as_object().is_some_and(|obj| !obj.is_empty()) {
        output.contents.push(data);
    }

    Ok(Json(output))
}
