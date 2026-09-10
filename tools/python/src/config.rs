#[derive(Debug, Default, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct PythonToolConfig {
    pub use_latest_build: bool,
}
