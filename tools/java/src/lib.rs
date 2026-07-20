pub mod config;
mod foojay;
mod java;
#[cfg(feature = "wasm")]
mod proto;
pub mod version;

#[cfg(feature = "wasm")]
pub use proto::*;
