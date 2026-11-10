mod paths;
pub mod registry;
mod versions;

pub use paths::*;
pub use versions::*;

/// Return true if the plugin is targeting a platform other than the machine it's
/// running on, for example, when proto locks other platforms. The host environment
/// then describes the targeted platform, but commands and files still describe the
/// machine, so they must not be used to inspect the platform.
///
/// The `foreign` field is read from the host environment directly, as the PDK
/// doesn't have it yet. Versions of proto without it never target another platform.
#[cfg(feature = "wasm")]
pub fn is_foreign_platform() -> bool {
    use extism_pdk::{config, json};

    #[derive(serde::Deserialize)]
    struct ForeignHost {
        #[serde(default)]
        foreign: bool,
    }

    config::get("host_environment")
        .ok()
        .flatten()
        .and_then(|value| json::from_str::<ForeignHost>(&value).ok())
        .is_some_and(|host| host.foreign)
}

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
