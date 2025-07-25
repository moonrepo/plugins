use proto_pdk::{WarpgateTracingOptions, get_test_environment, initialize_tracing_with_options};
use std::sync::atomic::{AtomicBool, Ordering};

static TRACING_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable_tracing() {
    // Disable tracing while running tests!
    if let Ok(Some(_)) = get_test_environment() {
        return;
    }

    // Abort early if already enabled. This can happen
    // when toolchains import tools, since this function
    // gets called multiple times!
    if TRACING_ENABLED.load(Ordering::Relaxed) {
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

    TRACING_ENABLED.store(true, Ordering::Release);
}
