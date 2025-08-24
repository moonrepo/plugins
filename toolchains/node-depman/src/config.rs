use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::Config;

config_struct!(
    /// Configures and enables the npm toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#npm
    #[derive(Config)]
    pub struct NpmToolchainConfig {
        /// List of arguments to append to `npm install` commands.
        /// These arguments are inherited by the JavaScript toolchain.
        pub install_args: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    /// Configures and enables the pnpm toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#pnpm
    #[derive(Config)]
    pub struct PnpmToolchainConfig {
        /// List of arguments to append to `pnpm install` commands.
        /// These arguments are inherited by the JavaScript toolchain.
        pub install_args: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    /// Configures and enables the Yarn toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#yarn
    #[derive(Config)]
    pub struct YarnToolchainConfig {
        /// List of arguments to append to `yarn install` commands.
        /// These arguments are inherited by the JavaScript toolchain.
        pub install_args: Vec<String>,

        /// List of plugins to automatically install for Yarn v2 and above.
        pub plugins: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
