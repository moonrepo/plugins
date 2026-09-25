use crate::metadata::DEFAULT_METADATA_URL;

#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct DotnetToolConfig {
    /// Base URL for release metadata, for mirrors and air-gapped networks.
    /// `{channel}/releases.json` and `releases-index.json` hang off it.
    pub metadata_url: String,

    /// Overrides the archive URL taken from the release metadata. Supports
    /// `{version}`, `{rid}` and `{extension}` tokens. When set, the metadata's
    /// checksum no longer applies and is not used.
    pub dist_url: Option<String>,
}

impl Default for DotnetToolConfig {
    fn default() -> Self {
        Self {
            metadata_url: DEFAULT_METADATA_URL.into(),
            dist_url: None,
        }
    }
}
