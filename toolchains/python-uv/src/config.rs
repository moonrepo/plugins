use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::Config;

config_struct!(
    /// Configures and enables the Python uv toolchain.
    #[derive(Config)]
    pub struct UvToolchainConfig {
        /// List of arguments to append to `uv sync` commands.
        /// These arguments are inherited by the Python toolchain.
        pub sync_args: Vec<String>,

        /// List of arguments to append to `uv venv` commands.
        /// These arguments are inherited by the Python toolchain.
        pub venv_args: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
