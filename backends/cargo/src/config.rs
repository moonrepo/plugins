use schematic::Schematic;
use serde::{Deserialize, Serialize};

/// Configuration for the `cargo` backend plugin.
#[derive(Debug, Default, Deserialize, Serialize, Schematic)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct CargoBackendConfig {
    /// Do not use `cargo-binstall` even when available.
    pub no_binstall: bool,

    /// Custom crate registry to resolve from.
    pub registry: Option<String>,
}

/// Configuration for the tool within the `cargo` backend plugin.
#[derive(Debug, Default, Deserialize, Serialize, Schematic)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct CargoToolConfig {
    /// The explicit binary within the package to install.
    pub bin: Option<String>,

    /// List of features to enable for the package.
    pub features: Vec<String>,

    /// Custom Git URL to the package.
    pub git_url: Option<String>,

    /// Disable the `default` feature of the package.
    pub no_default_features: bool,

    /// Custom crate registry to resolve packages from.
    pub registry: Option<String>,
}
