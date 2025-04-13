#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::host_log;
use moon_pdk_api::{AnyResult, toml_config};
use rust_tool::ToolchainToml as BaseToolchainToml;
use starbase_utils::toml::TomlValue;

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

toml_config!(ToolchainToml, BaseToolchainToml);

impl ToolchainToml {
    fn save_field(
        &self,
        _field: &str,
        _current_value: Option<&TomlValue>,
    ) -> AnyResult<Option<TomlValue>> {
        Ok(None)
    }
}
