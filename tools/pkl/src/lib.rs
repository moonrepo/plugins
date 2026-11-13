#[cfg(feature = "wasm")]
mod proto;

#[cfg(feature = "wasm")]
pub use proto::*;
