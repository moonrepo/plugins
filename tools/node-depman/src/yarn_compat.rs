use rustc_hash::FxHashMap;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct YarnAuthSettings {
    pub npm_always_auth: bool,
    pub npm_auth_ident: Option<String>,
    pub npm_auth_token: Option<String>,
    pub npm_publish_registry: Option<String>,
    pub npm_registry_server: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct YarnRcYaml {
    pub npm_auth_ident: Option<String>,
    pub npm_auth_token: Option<String>,
    pub npm_publish_registry: Option<String>,
    pub npm_registries: FxHashMap<String, YarnAuthSettings>,
    pub npm_registry_server: Option<String>,
    pub npm_scopes: FxHashMap<String, YarnAuthSettings>,
}
