#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct KotlinToolConfig {
    pub dist_url: String,
}

impl Default for KotlinToolConfig {
    fn default() -> Self {
        Self {
            dist_url: "https://github.com/JetBrains/kotlin/releases/download/{tag}/{file}".into(),
        }
    }
}
