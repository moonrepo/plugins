use moon_config::BinEntry;
use moon_pdk_api::{UnresolvedVersionSpec, Version, config_struct};
use schematic::Config;

config_struct!(
    /// Configures and enables the Rust toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#rust
    #[derive(Config)]
    pub struct RustToolchainConfig {
        /// When `version` is defined, syncs the version as a constraint to
        /// `Cargo.toml` under the `workspace.package.rust-version` or
        /// `package.rust-version` fields.
        pub add_msrv_constraint: bool,

        /// List of binaries to install into the environment using `cargo binstall`.
        #[setting(nested)]
        pub bins: Vec<BinEntry>,

        /// The version of `cargo-binstall` to install. Defaults to "latest" if not defined.
        pub binstall_version: Option<Version>,

        /// List of Rust components to automatically install with `rustup`.
        pub components: Vec<String>,

        /// When `version` is defined, syncs the version to `rust-toolchain.toml`
        /// under the `toolchain.channel` field.
        pub sync_toolchain_config: bool,

        /// List of Rust targets to automatically install with `rustup`.
        pub targets: Vec<String>,

        /// Configured version (channel) to download and install with `rustup`.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
