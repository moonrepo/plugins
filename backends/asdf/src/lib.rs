#[cfg(feature = "wasm")]
mod config;

#[cfg(feature = "wasm")]
mod proto;

#[cfg(feature = "wasm")]
pub use proto::*;
