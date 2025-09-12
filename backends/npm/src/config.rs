use schematic::Schematic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Schematic)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct NpmBackendConfig {
    pub bun: bool,
}
