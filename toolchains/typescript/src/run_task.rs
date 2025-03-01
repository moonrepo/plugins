use extism_pdk::json;
use starbase_utils::json::{JsonMap, JsonValue};
use typescript_tsconfig_json::CompilerOptions;

/// Hash compiler options that may alter compiled/generated output.
pub fn hash_compiler_options(compiler_options: &CompilerOptions) -> JsonValue {
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
