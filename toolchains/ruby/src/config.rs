use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::config_struct;
use schematic::{Config, ConfigEnum, derive_enum};

derive_enum!(
    /// The available dependency managers for Ruby.
    ///
    /// Bundler is universal today, but we model this so a future
    /// tool (eg. the "uv of Ruby") is a non-breaking addition rather
    ///  than a config rewrite.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum RubyPackageManager {
        #[default]
        Bundler,
    }
);

config_struct!(
    /// Configures and enables the Ruby toolchain.
    #[derive(Config)]
    pub struct RubyToolchainConfig {
        /// The dependency manager to use for installing gems,
        /// running inferred tasks, and much more.
        #[setting(default)]
        pub package_manager: RubyPackageManager,

        /// Where Bundler installs gems, relative to the dependency root.
        #[setting(default = "vendor/bundle")]
        pub bundle_path: String,

        /// Extra arguments appended to `bundle install`.
        pub bundler_install_args: Vec<String>,

        /// Run installs in frozen/deployment mode (CI-friendly).
        pub frozen: bool,

        /// Gem groups to exclude during a production install, applied via
        /// `BUNDLE_WITHOUT`. Defaults to development + test when empty.
        pub production_without_groups: Vec<String>,

        /// Configured version to download and install.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
