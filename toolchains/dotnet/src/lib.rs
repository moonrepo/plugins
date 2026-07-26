pub mod config;
pub mod discovery;
pub mod dotnet_install;
pub mod eval_cache;
pub mod global_json;
pub mod infer_tasks;
pub mod inherited_tasks;
pub mod msbuild;
pub mod nuget_lock;

#[cfg(feature = "wasm")]
mod project_graph;
#[cfg(feature = "wasm")]
mod tier1;
#[cfg(feature = "wasm")]
mod tier2;
#[cfg(feature = "wasm")]
mod tier2_env;
#[cfg(feature = "wasm")]
mod tier3;

#[cfg(feature = "wasm")]
pub use project_graph::*;
#[cfg(feature = "wasm")]
pub use tier1::*;
#[cfg(feature = "wasm")]
pub use tier2::*;
#[cfg(feature = "wasm")]
pub use tier2_env::*;
#[cfg(feature = "wasm")]
pub use tier3::*;
