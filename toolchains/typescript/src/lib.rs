#[cfg(feature = "wasm")]
pub mod config;
#[cfg(feature = "wasm")]
pub mod run_task;
#[cfg(feature = "wasm")]
pub mod sync_project;
pub mod tsconfig_json;

#[cfg(feature = "wasm")]
mod moon;

#[cfg(feature = "wasm")]
pub use moon::*;
