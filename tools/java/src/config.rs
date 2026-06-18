#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct JavaToolConfig {
    pub api_url: String,
    pub image_type: String,
    pub release_type: String,
    pub vendor: String,
}

impl Default for JavaToolConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.foojay.io/disco/v3.0".into(),
            image_type: "jdk".into(),
            release_type: "ga".into(),
            vendor: "temurin".into(),
        }
    }
}
