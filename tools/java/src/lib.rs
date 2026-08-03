#[cfg(feature = "wasm")]
mod config;
#[cfg(feature = "wasm")]
mod foojay;
mod java;
#[cfg(feature = "wasm")]
mod proto;
pub mod version;

#[cfg(feature = "wasm")]
pub use proto::*;
