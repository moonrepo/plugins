#[cfg(feature = "wasm")]
mod proto;
mod version;

#[cfg(feature = "wasm")]
pub use proto::*;
