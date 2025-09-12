#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct GoToolConfig {
    pub dist_url: String,
    pub gobin: bool,
}

impl Default for GoToolConfig {
    fn default() -> Self {
        Self {
            dist_url: "https://dl.google.com/go/{file}".into(),
            gobin: false,
        }
    }
}
