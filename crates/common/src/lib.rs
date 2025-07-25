use proto_pdk::{WarpgateTracingOptions, get_test_environment, initialize_tracing_with_options};

pub fn enable_tracing() {
    // Disable tracing while running tests
    if let Ok(Some(_)) = get_test_environment() {
        return;
    }

    initialize_tracing_with_options(WarpgateTracingOptions {
        modules: vec![
            "moon".into(),
            "nodejs_package_json".into(),
            "proto".into(),
            "starbase".into(),
            "typescript_tsconfig_json".into(),
        ],
        ..Default::default()
    });
}
