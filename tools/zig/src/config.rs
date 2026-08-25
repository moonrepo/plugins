#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct ZigToolConfig {
    pub index_url: String,
}

impl Default for ZigToolConfig {
    fn default() -> Self {
        Self {
            index_url: "https://ziglang.org/download/index.json".into(),
        }
    }
}
