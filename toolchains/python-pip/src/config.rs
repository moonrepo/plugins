use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::Config;

config_struct!(
    /// Configures and enables the Python pip toolchain.
    #[derive(Config)]
    pub struct PipToolchainConfig {
        /// List of arguments to append to `pip install` commands, and to
        /// `uv pip install` commands when the package manager is `uv-pip`.
        /// These arguments are inherited by the Python toolchain.
        pub install_args: Vec<String>,

        /// List of arguments to append to `python venv` commands.
        /// These arguments are inherited by the Python toolchain.
        pub venv_args: Vec<String>,

        /// Unused. The venv uses the pip that ships with Python.
        #[deprecated]
        pub version: Option<UnresolvedVersionSpec>,
    }
);
