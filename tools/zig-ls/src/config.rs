#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct ZlsToolConfig {
    pub index_url: String,
}

impl Default for ZlsToolConfig {
    fn default() -> Self {
        Self {
            index_url: "https://releases.zigtools.org/v1/zls/index.json".into(),
        }
    }
}
