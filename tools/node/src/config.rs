#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct NodeToolConfig {
    pub bundled_npm: bool,
    pub dist_url: String,
}

impl Default for NodeToolConfig {
    fn default() -> Self {
        Self {
            bundled_npm: false,
            dist_url: "https://nodejs.org/download/release/v{version}/{file}".into(),
        }
    }
}
