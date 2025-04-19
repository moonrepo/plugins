#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct DenoPluginConfig {
    pub dist_url: String,
}

impl Default for DenoPluginConfig {
    fn default() -> Self {
        Self {
            dist_url: "https://github.com/denoland/deno/releases/download/v{version}/{file}".into(),
        }
    }
}
