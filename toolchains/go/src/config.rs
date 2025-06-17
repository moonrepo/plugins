use moon_pdk_api::{UnresolvedVersionSpec, config_struct};
use schematic::Config;

config_struct!(
    /// Configures and enables the Go toolchain.
    #[derive(Config)]
    pub struct GoToolchainConfig {
        /// Configured version to download and install`.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
