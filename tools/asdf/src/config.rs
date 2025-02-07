#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]

/// https://asdf-vm.com/manage/plugins.html
pub struct AsdfPluginConfig {
    pub asdf_shortname: Option<String>,
	pub asdf_repository: Option<String>,
    pub executable_name: Option<String>
}

impl Default for AsdfPluginConfig {
    fn default() -> Self {
        Self {
            asdf_shortname: None,
			asdf_repository: None,
            executable_name: None
        }
    }
}