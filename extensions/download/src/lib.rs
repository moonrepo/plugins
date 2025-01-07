#[cfg(feature = "wasm")]
mod download_ext;

#[cfg(feature = "wasm")]
pub use download_ext::*;
