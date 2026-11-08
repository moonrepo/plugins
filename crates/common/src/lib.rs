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

    install_panic_hook();

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

/// Pass panic messages to Extism as the call's error, so the host shows them
/// instead of a WASM trap. A panic that traps never finishes, and `set_hook`
/// panics while panicking, so this must run before the first panic.
#[cfg(feature = "wasm")]
fn install_panic_hook() {
    use std::sync::Once;

    static INSTALLED: Once = Once::new();

    if std::thread::panicking() {
        return;
    }

    INSTALLED.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            if let Ok(memory) = extism_pdk::Memory::from_bytes(info.to_string()) {
                unsafe { extism_pdk::extism::error_set(memory.offset()) };
            }
        }));
    });
}
