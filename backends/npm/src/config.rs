use schematic::Schematic;
use serde::{Deserialize, Serialize};

/// Configuration for the `npm` backend plugin.
#[derive(Debug, Default, Deserialize, Serialize, Schematic)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct NpmBackendConfig {
    /// Use `bun` for installing packages instead of `npm` and `node`.
    pub bun: bool,
}
