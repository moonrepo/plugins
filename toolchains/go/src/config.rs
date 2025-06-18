use moon_config::BinEntry;
use moon_pdk_api::{UnresolvedVersionSpec, config_struct};
use schematic::Config;

config_struct!(
    /// Configures and enables the Go toolchain.
    #[derive(Config)]
    pub struct GoToolchainConfig {
        /// List of binaries to install into the environment using `go install`.
        #[setting(nested)]
        pub bins: Vec<BinEntry>,

        /// Tidy modules when dependencies or `go.sum` changes
        /// by running `go mod tidy`.
        pub tidy_on_change: bool,

        /// Configured version to download and install`.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
