#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct BunToolConfig {
    pub dist_url: String,
}

impl Default for BunToolConfig {
    fn default() -> Self {
        Self {
            dist_url: "https://github.com/oven-sh/bun/releases/download/bun-v{version}/{file}"
                .into(),
        }
    }
}
