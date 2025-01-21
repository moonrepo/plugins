use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AsdfPluginConfig {
    #[serde(default)]
    pub asdf_plugin: Option<String>,
    #[serde(default)]
    pub asdf_repository: Option<String>,
    #[serde(default)]
    pub asdf_version: Option<String>,
} 
