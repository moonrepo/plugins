#[cfg(any(feature = "wasm", test))]
#[cfg_attr(not(feature = "wasm"), allow(dead_code, unused_imports))]
mod config;
#[cfg(any(feature = "wasm", test))]
#[cfg_attr(not(feature = "wasm"), allow(dead_code, unused_imports))]
mod foojay;
#[cfg(feature = "wasm")]
mod java;
#[cfg(feature = "wasm")]
mod proto;
pub mod version;

#[cfg(feature = "wasm")]
pub use proto::*;
