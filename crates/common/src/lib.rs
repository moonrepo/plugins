use proto_pdk::{WarpgateTracingOptions, initialize_tracing_with_options};

pub fn enable_tracing() {
    initialize_tracing_with_options(WarpgateTracingOptions {
        modules: vec![
            "moon".into(),
            "nodejs_package_json".into(),
            "proto".into(),
            "starbase".into(),
            "typescript_tsconfig_json".into(),
        ],
        ..Default::default()
    })
}
