use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::{Config, ConfigEnum, derive_enum};

derive_enum!(
    /// The available version managers for Node.js.
    #[derive(ConfigEnum, Copy)]
    pub enum NodeVersionManager {
        Nodenv,
        Nvm,
    }
);

derive_enum!(
    /// The type of profiling operation to use.
    #[derive(ConfigEnum, Copy)]
    pub enum NodeProfileType {
        Cpu,
        Heap,
    }
);

config_struct!(
    /// Configures and enables the Node.js toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#node
    #[derive(Config)]
    pub struct NodeToolchainConfig {
        /// List of arguments to pass to all `node` executions, when configured as a
        /// task `command`. Arguments will be appended after the `node` executable,
        /// but before other arguments.
        pub execute_args: Vec<String>,

        /// Enable the v8 profiler for all `node` executions, when configured as a
        /// task `command`. Note: This should only be temporarily enabled for debugging
        /// purposes, and not always enabled!
        pub profile_execution: Option<NodeProfileType>,

        /// When `version` is defined, syncs the version to the chosen config.
        pub sync_version_manager_config: Option<NodeVersionManager>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
