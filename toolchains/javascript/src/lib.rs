mod config;
#[cfg(feature = "wasm")]
mod tier1;

#[cfg(feature = "wasm")]
pub use tier1::*;
