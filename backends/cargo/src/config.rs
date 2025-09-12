use schematic::Schematic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Schematic)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct CargoBackendConfig {
    pub registry: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, Schematic)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct CargoToolConfig {
    pub features: Vec<String>,
    pub git_url: Option<String>,
    pub no_default_features: bool,
    pub registry: Option<String>,
}
