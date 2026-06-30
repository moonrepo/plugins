use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::Config;

config_struct!(
    /// Configures and enables the Python Poetry toolchain.
    #[derive(Config)]
    pub struct PoetryToolchainConfig {
        /// List of arguments to append to `poetry install` commands.
        /// These arguments are inherited by the Python toolchain.
        pub install_args: Vec<String>,

        /// List of arguments to append to `python venv` commands.
        /// These arguments are inherited by the Python toolchain.
        pub venv_args: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
