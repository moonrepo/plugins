/// https://asdf-vm.com/manage/plugins.html
#[derive(Debug, Default, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct AsdfPluginConfig {
    pub asdf_shortname: Option<String>,
    pub asdf_repository: Option<String>,
    pub executable_name: Option<String>,
}
