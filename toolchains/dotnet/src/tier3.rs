// Installation is the proto tool's job. Re-exporting it here means the one
// wasm binary serves as both a moon toolchain and a proto tool, which is how
// every other paired plugin in this repository is wired, and it is what makes
// `setup_toolchain` unnecessary: moon drives the proto flow itself.
pub use dotnet_tool::*;
