use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::{Config, ConfigEnum};
use serde::{Deserialize, Serialize};

/// The available version managers for Node.js.
#[derive(ConfigEnum, Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum NodeVersionManager {
    Nodenv,
    Nvm,
}

/// The type of profiling operation to use.
#[derive(ConfigEnum, Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum NodeProfileType {
    Cpu,
    Heap,
}

config_struct!(
    /// Configures and enables the Node.js toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#node
    #[derive(Config)]
    pub struct NodeToolchainConfig {
        /// List of arguments to pass to all `node` executions (via task commands).
        /// Arguments will be appended after `node` but before other arguments.
        pub execute_args: Vec<String>,

        /// Enable the v8 profiler for all `node` executions (via task commands).
        /// Note: This should only be enabled for debugging purposes!
        pub profile_execution: Option<NodeProfileType>,

        /// When `version` is defined, syncs the version to the chosen config.
        pub sync_version_manager_config: Option<NodeVersionManager>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
