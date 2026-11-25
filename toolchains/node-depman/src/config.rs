use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::Config;

config_struct!(
    /// Configures and enables the npm toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#npm
    #[derive(Config)]
    pub struct NpmToolchainConfig {
        /// List of arguments to append to `npm install`, `npm ci` and `npm dedupe`
        /// commands. These arguments are inherited by the JavaScript toolchain.
        pub install_args: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    /// Configures and enables the Nub toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#unstable_nub
    #[derive(Config)]
    pub struct NubToolchainConfig {
        /// List of arguments to append to `nub install` commands.
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
        /// These arguments are inherited by the JavaScript toolchain, but not
        /// by `pnpm dedupe`, so layout settings like `nodeLinker` and
        /// `shamefullyHoist` belong in `pnpm-workspace.yaml`.
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
        /// These arguments are inherited by the JavaScript toolchain, but not
        /// by `yarn workspaces focus`, which focused and production installs
        /// run on Yarn 2 and later.
        pub install_args: Vec<String>,

        /// Unused. Yarn 4 and later bundle the official plugins. For Yarn 2
        /// and 3, commit `.yarn/plugins` and `.yarnrc.yml`.
        #[deprecated]
        pub plugins: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
