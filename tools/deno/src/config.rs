#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct DenoToolConfig {
    pub dist_url: String,
}

impl Default for DenoToolConfig {
    fn default() -> Self {
        Self {
            dist_url: "https://github.com/denoland/deno/releases/download/v{version}/{file}".into(),
        }
    }
}
