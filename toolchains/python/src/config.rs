use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::{Config, ConfigEnum, derive_enum};

derive_enum!(
    /// The available package managers for Python.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum PythonPackageManager {
        #[default]
        Pip,
        // Poetry,
        Uv,
        UvPip,
    }
);

derive_enum!(
    /// The location in which to create the Python virtual environment.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum PythonVenvLocation {
        #[default]
        Project,
        Workspace,
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

        /// The location where to create the virtual environment,
        /// in which dependencies will be installed into.
        pub venv_location: PythonVenvLocation,

        /// The name of virtual environment folder name.
        #[setting(default = ".venv")]
        pub venv_name: String,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);

// This config represents shared package manager configuration that
// is loaded from external toolchains.
config_struct!(
    #[derive(Default)]
    pub struct SharedPackageManagerConfig {
        #[serde(alias = "syncArgs")] // uv
        pub install_args: Vec<String>,
        pub version: Option<UnresolvedVersionSpec>,
    }
);
