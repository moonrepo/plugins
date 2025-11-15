use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::{Config, ConfigEnum, derive_enum};

derive_enum!(
    /// The available package managers for Python.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum PythonPackageManager {
        #[default]
        Pip,
        Poetry,
        Uv,
    }
);

config_struct!(
    /// Configures and enables the Python toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#python
    #[derive(Config)]
    pub struct PythonToolchainConfig {
        /// The package manager to use for installing dependencies,
        /// running inferred tasks, and much more.
        pub package_manager: Option<PythonPackageManager>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
