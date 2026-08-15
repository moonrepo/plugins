#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct SwiftToolConfig {
    pub dist_url: String,
    pub linux_archive_suffix: String,
    pub linux_platform: String,
}

impl Default for SwiftToolConfig {
    fn default() -> Self {
        Self {
            dist_url: "https://download.swift.org/{release}/{platform}/{folder}/{file}".into(),
            linux_archive_suffix: "ubuntu24.04".into(),
            linux_platform: "ubuntu2404".into(),
        }
    }
}
