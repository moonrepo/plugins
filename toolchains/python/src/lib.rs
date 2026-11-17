pub mod config;
#[cfg(feature = "wasm")]
mod managers;
// Pure logic, only used by the plugin functions (ADR-0006.4)
#[cfg_attr(not(feature = "wasm"), allow(dead_code, unused_imports))]
mod pyproject_toml;
#[cfg_attr(not(feature = "wasm"), allow(dead_code))]
mod requirements;
#[cfg(feature = "wasm")]
mod tier1;
#[cfg(feature = "wasm")]
mod tier2;
#[cfg(feature = "wasm")]
mod tier3;
#[cfg_attr(not(feature = "wasm"), allow(dead_code))]
mod venv;

#[cfg(feature = "wasm")]
pub use tier1::*;
#[cfg(feature = "wasm")]
pub use tier2::*;
#[cfg(feature = "wasm")]
pub use tier3::*;
