pub mod config;
pub mod go_mod;
pub mod go_sum;
pub mod go_work;

#[cfg(feature = "wasm")]
mod tier1;
#[cfg(feature = "wasm")]
mod tier2;
#[cfg(feature = "wasm")]
mod tier3;

#[cfg(feature = "wasm")]
pub use tier1::*;
#[cfg(feature = "wasm")]
pub use tier2::*;
#[cfg(feature = "wasm")]
pub use tier3::*;
