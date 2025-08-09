use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::Config;

config_struct!(
    /// Configures and enables the Bun toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#bun
    #[derive(Config)]
    pub struct BunToolchainConfig {
        /// List of arguments to pass to all `bun` executions, when configured as a
        /// task `command`. Arguments will be appended after the `bun` executable,
        /// but before other arguments.
        pub execute_args: Vec<String>,

        /// List of arguments to append to `bun install` commands.
        /// These arguments are inherited by the JavaScript toolchain.
        pub install_args: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
