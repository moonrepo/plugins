use starbase_utils::json::{JsonMap, JsonValue};
use typescript_tsconfig_json::CompilerOptions;

/// Hash compiler options that may alter compiled/generated output.
pub fn hash_compiler_options(compiler_options: &CompilerOptions) -> JsonValue {
    let mut options = JsonMap::default();

    if let Some(jsx) = &compiler_options.jsx {
        options.insert("jsx".into(), format!("{jsx:?}").into());
    }

    if let Some(jsx_factory) = &compiler_options.jsx_factory {
        options.insert("jsxFactory".into(), format!("{jsx_factory}").into());
    }

    if let Some(jsx_fragment_factory) = &compiler_options.jsx_fragment_factory {
        options.insert(
            "jsxFragmentFactory".into(),
            format!("{jsx_fragment_factory}").into(),
        );
    }

    if let Some(jsx_import_source) = &compiler_options.jsx_import_source {
        options.insert(
            "jsxImportSource".into(),
            format!("{jsx_import_source}").into(),
        );
    }

    if let Some(module) = &compiler_options.module {
        options.insert("module".into(), format!("{module:?}").into());
    }

    if let Some(module_resolution) = &compiler_options.module_resolution {
        options.insert(
            "moduleResolution".into(),
            format!("{module_resolution:?}").into(),
        );
    }

    if let Some(target) = &compiler_options.target {
        options.insert("target".into(), format!("{target:?}").into());
    }

    JsonValue::Object(options)
}
