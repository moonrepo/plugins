// `deno.json`, `deno.jsonc`

#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::{HostLogInput, host_log};
use moon_pdk_api::{AnyResult, json_config};
use serde::{Deserialize, Serialize};
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

json_config!(DenoJson, BaseDenoJson);

impl DenoJson {
    #[allow(dead_code, unused_variables)]
    fn save_field(&self, field: &str, config: &mut JsonValue) -> AnyResult<()> {
        let Some(_root) = config.as_object_mut() else {
            return Ok(());
        };

        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Setting <property>{field}</file> in <path>{}</path>",
                self.path,
            );
        }

        Ok(())
    }
}

// https://github.com/denoland/deno/blob/main/cli/schemas/config-file.v1.json
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct BaseDenoJson {
    pub exports: DenoJsonExports,
    pub imports: BTreeMap<String, String>,
    pub import_map: Option<String>,
    pub links: Vec<String>,
    pub lock: DenoJsonLock,
    pub scopes: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DenoJsonLock {
    Enabled(bool),
    Name(String),
    Config {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        frozen: bool,
    },
}

impl Default for DenoJsonLock {
    fn default() -> Self {
        Self::Enabled(true)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DenoJsonExports {
    Main(String),
    Paths(BTreeMap<String, String>),
}

impl Default for DenoJsonExports {
    fn default() -> Self {
        Self::Paths(BTreeMap::default())
    }
}
