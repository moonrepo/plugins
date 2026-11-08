pub mod config;
#[cfg(feature = "wasm")]
mod infer_tasks;
#[cfg(any(feature = "wasm", test))]
#[cfg_attr(not(feature = "wasm"), allow(dead_code, unused_imports))]
mod lockfiles;
pub mod package_json;
#[cfg(feature = "wasm")]
mod tier1;
#[cfg(feature = "wasm")]
mod tier2;

#[cfg(feature = "wasm")]
pub use tier1::*;
#[cfg(feature = "wasm")]
pub use tier2::*;
