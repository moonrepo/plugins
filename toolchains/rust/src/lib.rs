pub mod cargo_metadata;
pub mod cargo_toml;
mod config;
pub mod toolchain_toml;

#[cfg(feature = "wasm")]
mod install_deps;
#[cfg(feature = "wasm")]
mod moon;
#[cfg(feature = "wasm")]
mod run_task;
#[cfg(feature = "wasm")]
mod setup_env;
#[cfg(feature = "wasm")]
mod tier1;
#[cfg(feature = "wasm")]
mod tier3;

#[cfg(feature = "wasm")]
pub use install_deps::*;
#[cfg(feature = "wasm")]
pub use moon::*;
#[cfg(feature = "wasm")]
pub use run_task::*;
#[cfg(feature = "wasm")]
pub use setup_env::*;
#[cfg(feature = "wasm")]
pub use tier1::*;
#[cfg(feature = "wasm")]
pub use tier3::*;
