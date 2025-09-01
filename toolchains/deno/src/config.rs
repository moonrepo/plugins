use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::Config;

config_struct!(
    /// Configures and enables the Deno toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#deno
    #[derive(Config)]
    pub struct DenoToolchainConfig {
        /// List of arguments to pass to all `deno` executions, when configured as a
        /// task `command`. Arguments will be appended after the `deno` executable,
        /// but before other arguments.
        pub execute_args: Vec<String>,

        /// List of arguments to append to `deno install` commands.
        /// These arguments are inherited by the JavaScript toolchain.
        pub install_args: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
