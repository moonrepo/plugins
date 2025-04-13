use moon_config::BinEntry;
use moon_pdk_api::config_struct;
use schematic::Schematic;
use semver::Version;

config_struct!(
    /// Configures and enables the Rust platform.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#rust
    #[derive(Schematic)]
    pub struct RustToolchainConfig {
        /// When `version` is defined, syncs the version as a constraint to
        /// `Cargo.toml` under the `package.rust-version` field.
        pub add_msrv_constraint: bool,

        /// List of binaries to install into the environment using `cargo binstall`.
        #[schema(nested)]
        pub bins: Vec<BinEntry>,

        /// The version of `cargo-binstall` to install. Defaults to latest if not defined.
        pub binstall_version: Option<Version>,

        /// List of Rust components to automatically install.
        pub components: Vec<String>,

        /// A relative path from the moon workspace root, to the root of the Cargo
        /// workspace that contains a `Cargo.lock` file.
        #[schema(default = ".")]
        pub root: String,

        /// When `version` is defined, syncs the version to `rust-toolchain.toml`.
        pub sync_toolchain_config: bool,

        /// List of Rust targets to automatically install.
        pub targets: Vec<String>,
    }
);

impl Default for RustToolchainConfig {
    fn default() -> Self {
        Self {
            add_msrv_constraint: false,
            bins: vec![],
            binstall_version: None,
            components: vec![],
            root: ".".into(),
            sync_toolchain_config: false,
            targets: vec![],
        }
    }
}
