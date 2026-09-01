#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct NodeToolConfig {
    pub bundled_npm: bool,
    pub dist_url: String,
    pub dist_url_unofficial: String,
    pub index_url: String,
}

impl Default for NodeToolConfig {
    fn default() -> Self {
        Self {
            bundled_npm: false,
            dist_url: "https://nodejs.org/download/{channel}/v{version}/{file}".into(),
            dist_url_unofficial:
                "https://unofficial-builds.nodejs.org/download/{channel}/v{version}/{file}".into(),
            index_url: "https://nodejs.org/download/{channel}/index.json".into(),
        }
    }
}
