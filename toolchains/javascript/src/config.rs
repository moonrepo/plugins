use moon_common::Id;
use moon_pdk_api::{MatchesVersion, Requirement, UnresolvedVersionSpec, Version, config_struct};
use nodejs_package_json::VersionProtocol;
use rustc_hash::FxHashMap;
use schematic::{Config, ConfigEnum, derive_enum};

derive_enum!(
    /// The available package managers for JavaScript.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum JavaScriptPackageManager {
        Bun,
        Deno,
        #[default]
        Npm,
        Nub,
        Pnpm,
        Yarn,
    }
);

impl JavaScriptPackageManager {
    pub fn get_runtime_toolchain(&self) -> Id {
        match self {
            Self::Bun => Id::raw("bun"),
            Self::Deno => Id::raw("deno"),
            Self::Nub => Id::raw("nub"),
            _ => Id::raw("node"),
        }
    }

    /// Installs dependencies for the Node.js ecosystem.
    pub fn is_for_node(&self) -> bool {
        matches!(self, Self::Npm | Self::Nub | Self::Pnpm | Self::Yarn)
    }

    /// Runs as a standalone binary and does not require the
    /// Node.js toolchain to operate.
    pub fn is_standalone(&self) -> bool {
        matches!(self, Self::Bun | Self::Deno | Self::Nub)
    }

    /// The lockfiles that this package manager writes.
    pub fn get_lock_file_names(&self) -> &'static [&'static str] {
        match self {
            Self::Bun => &["bun.lock", "bun.lockb"],
            Self::Deno => &["deno.lock"],
            Self::Npm => &["package-lock.json", "npm-shrinkwrap.json"],
            Self::Nub => &["nub.lock"],
            Self::Pnpm => &["pnpm-lock.yaml"],
            Self::Yarn => &["yarn.lock"],
        }
    }
}

derive_enum!(
    /// Formats that a local workspace `package.json` dependency version can be.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum JavaScriptDependencyVersionFormat {
        File,         // file:..
        Link,         // link:..
        Star,         // *
        Version,      // 0.0.0
        VersionCaret, // ^0.0.0
        VersionTilde, // ~0.0.0
        #[default]
        Workspace, // workspace:*
        WorkspaceCaret, // workspace:^
        WorkspaceTilde, // workspace:~
    }
);

impl JavaScriptDependencyVersionFormat {
    pub fn get_prefix(&self) -> String {
        match self {
            Self::File => "file:".into(),
            Self::Link => "link:".into(),
            Self::Star => "*".into(),
            Self::Version => "".into(),
            Self::VersionCaret => "^".into(),
            Self::VersionTilde => "~".into(),
            Self::Workspace => "workspace:*".into(),
            Self::WorkspaceCaret => "workspace:^".into(),
            Self::WorkspaceTilde => "workspace:~".into(),
        }
    }

    pub fn get_supported_for(&self, pm: &JavaScriptPackageManager) -> Self {
        match pm {
            JavaScriptPackageManager::Bun => {
                if matches!(self, Self::WorkspaceCaret | Self::WorkspaceTilde) {
                    Self::Workspace
                } else {
                    *self
                }
            }
            JavaScriptPackageManager::Npm => {
                if matches!(
                    self,
                    Self::Link | Self::Workspace | Self::WorkspaceCaret | Self::WorkspaceTilde
                ) {
                    Self::File
                } else {
                    *self
                }
            }
            _ => *self,
        }
    }
}

config_struct!(
    /// Configures and enables the JavaScript toolchain.
    #[derive(Config)]
    pub struct JavaScriptToolchainConfig {
        /// Automatically dedupes the lockfile when dependencies have changed.
        /// This will run locally after dependencies have been installed.
        #[setting(default = true)]
        pub dedupe_on_lockfile_change: bool,

        /// The dependency version format to use when syncing projects
        /// as workspace dependencies.
        pub dependency_version_format: JavaScriptDependencyVersionFormat,

        /// Automatically infer moon tasks from `package.json` scripts or
        /// `deno.json` tasks. Some caveats to be aware of:
        ///
        /// - Lifecycle (pre, post) scripts are not inferred.
        /// - Outputs will be automatically detected (if possible).
        /// - Script names that contain `:` will be converted to `-`.
        /// - Script names that contain `dev`, `start`, `serve`, or `preview`
        ///   will be considered local only.
        pub infer_tasks_from_scripts: bool,

        /// The package manager to use for installing dependencies,
        /// running inferred tasks, and much more.
        pub package_manager: Option<JavaScriptPackageManager>,

        /// Enforces that only the root `package.json` can be used for dependencies,
        /// which supports the "one version policy" pattern.
        pub root_package_dependencies_only: bool,

        /// Automatically syncs the configured package manager version
        /// to the root `packageManager` field in `package.json`.
        #[setting(default = true)]
        pub sync_package_manager_field: bool,

        /// Automatically syncs moon project-to-project relationships as
        /// dependencies for each `package.json` in the workspace.
        #[setting(default = true)]
        pub sync_project_workspace_dependencies: bool,
    }
);

// This config represents shared package manager configuration that
// is loaded from external toolchains, primarily `node_depman_toolchain`.
config_struct!(
    #[derive(Default)]
    pub struct SharedPackageManagerConfig {
        pub install_args: Vec<String>,
        pub version: Option<UnresolvedVersionSpec>,
    }
);

// The JavaScript toolchain's own config, for functions that don't receive it.
config_struct!(
    #[derive(Default)]
    pub struct LockConfig {
        pub package_manager: Option<JavaScriptPackageManager>,
    }
);

impl SharedPackageManagerConfig {
    pub fn version_satisfies(&self, req: &str) -> bool {
        let Some(spec) = &self.version else {
            return false;
        };

        let req = Requirement::parse(req).unwrap();

        let mut version = match spec {
            UnresolvedVersionSpec::Canary => return true,
            // These resolve to the newest release
            UnresolvedVersionSpec::Alias(alias) => return alias == "latest" || alias == "stable",
            UnresolvedVersionSpec::Requirement(value) => Version::new(
                value.major.unwrap_or(0),
                value.minor.unwrap_or(0),
                value.patch.unwrap_or(0),
            ),
            UnresolvedVersionSpec::Version(version) => version.clone(),
            _ => return false,
        };

        // A prerelease, like `6.0.0-rc.23`, has the features of its release,
        // but semver ranges like `>=2.0.0` don't match it
        version.prerelease = None;
        version.build = None;

        req.matches(&version)
    }
}

pub type CatalogsMap = FxHashMap<String, FxHashMap<String, VersionProtocol>>;

#[cfg(test)]
mod tests {
    use super::*;

    fn config(version: &str) -> SharedPackageManagerConfig {
        SharedPackageManagerConfig {
            version: Some(UnresolvedVersionSpec::parse(version).unwrap()),
            ..Default::default()
        }
    }

    #[test]
    fn satisfies_versions_and_requirements() {
        assert!(config("1.4.0").version_satisfies(">=1.4.0"));
        assert!(!config("1.3.9").version_satisfies(">=1.4.0"));
        assert!(config("2").version_satisfies(">=2.0.0"));
        assert!(!config("1").version_satisfies(">=2.0.0"));
    }

    #[test]
    fn satisfies_prereleases_like_their_release() {
        assert!(config("6.0.0-rc.23").version_satisfies(">=2.0.0"));
        assert!(config("1.4.0-canary.1").version_satisfies(">=1.4.0"));
        assert!(!config("1.3.0-rc.1").version_satisfies(">=1.4.0"));
    }

    #[test]
    fn satisfies_latest_aliases() {
        assert!(config("latest").version_satisfies(">=1.4.0"));
        assert!(config("stable").version_satisfies(">=1.4.0"));
        assert!(config("canary").version_satisfies(">=1.4.0"));
        assert!(!SharedPackageManagerConfig::default().version_satisfies(">=1.4.0"));
    }
}
