use moon_pdk_api::{UnresolvedVersionSpec, Version, VersionReq, config_struct};
use schematic::{Config, ConfigEnum};
use serde::{Deserialize, Serialize};

/// The available package managers for JavaScript.
#[derive(ConfigEnum, Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JavaScriptPackageManager {
    Bun,
    // Deno,
    #[default]
    Npm,
    Pnpm,
    Yarn,
}

impl JavaScriptPackageManager {
    pub fn is_for_node(&self) -> bool {
        matches!(self, Self::Npm | Self::Pnpm | Self::Yarn)
    }
}

/// Formats that a `package.json` dependency version can be.
#[derive(ConfigEnum, Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
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

impl JavaScriptDependencyVersionFormat {
    pub fn get_default_for(&self, pm: &JavaScriptPackageManager) -> Self {
        match pm {
            JavaScriptPackageManager::Npm => Self::File,
            _ => Self::Workspace,
        }
    }

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

    pub fn is_supported_by(&self, pm: &JavaScriptPackageManager) -> bool {
        match pm {
            JavaScriptPackageManager::Bun => {
                !matches!(self, Self::WorkspaceCaret | Self::WorkspaceTilde)
            }
            JavaScriptPackageManager::Npm => !matches!(
                self,
                Self::Link | Self::Workspace | Self::WorkspaceCaret | Self::WorkspaceTilde
            ),
            JavaScriptPackageManager::Pnpm => true,
            JavaScriptPackageManager::Yarn => true,
        }
    }
}

config_struct!(
    /// Configures and enables the JavaScript toolchain.
    #[derive(Config)]
    pub struct JavaScriptToolchainConfig {
        /// Automatically dedupes the lockfile when dependencies have changed.
        #[setting(default = true)]
        pub dedupe_on_lockfile_change: bool,

        /// The dependency version format to use when syncing projects
        /// as dependencies.
        pub dependency_version_format: JavaScriptDependencyVersionFormat,

        /// Automatically infer moon tasks from `package.json` scripts.
        pub infer_tasks_from_scripts: bool,

        /// The package manager to use for installing dependencies.
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

impl SharedPackageManagerConfig {
    pub fn version_satisfies(&self, req: &str) -> bool {
        let Some(spec) = &self.version else {
            return false;
        };

        let req = VersionReq::parse(req).unwrap();

        match spec {
            UnresolvedVersionSpec::Canary => true,
            UnresolvedVersionSpec::Req(value) => {
                let value = value.comparators.first().unwrap();
                let mut version = Version::new(
                    value.major,
                    value.minor.unwrap_or(0),
                    value.patch.unwrap_or(0),
                );
                version.pre = value.pre.clone();

                req.matches(&version)
            }
            UnresolvedVersionSpec::Calendar(version) => req.matches(version),
            UnresolvedVersionSpec::Semantic(version) => req.matches(version),
            _ => false,
        }
    }
}
