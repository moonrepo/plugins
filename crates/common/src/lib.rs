mod paths;
pub mod registry;
mod versions;

pub use paths::*;
pub use versions::*;

#[cfg(feature = "wasm")]
pub fn enable_tracing() {
    use proto_pdk::{
        WarpgateTracingOptions, get_test_environment, initialize_tracing_with_options,
    };
    use std::sync::atomic::{AtomicBool, Ordering};

    static TRACING_ENABLED: AtomicBool = AtomicBool::new(false);

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

#[cfg(all(feature = "wasm", not(feature = "tool-versions-fallback")))]
pub use proto_pdk::{load_git_tags, load_tool_versions};

#[cfg(feature = "tool-versions-fallback")]
mod tool_versions_fallback;

#[cfg(feature = "tool-versions-fallback")]
pub use tool_versions_fallback::*;
