use cargo_toml::Manifest as BaseCargoToml;
#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::host_log;
use moon_pdk_api::{AnyResult, toml_config};
use serde::{Deserialize, Serialize};
use starbase_utils::toml::{self, TomlValue};

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

toml_config!(CargoToml, CargoTomlInner);

impl CargoToml {
    fn save_field(
        &self,
        _field: &str,
        _current_value: Option<&TomlValue>,
    ) -> AnyResult<Option<TomlValue>> {
        Ok(None)
    }
}

// `cargo_toml::Manifest` does not implement `Default`,
// so we have this hacky workaround...
#[derive(Deserialize, Serialize)]
pub struct CargoTomlInner(BaseCargoToml);

impl Default for CargoTomlInner {
    fn default() -> Self {
        Self(toml::parse("").unwrap())
    }
}

impl std::ops::Deref for CargoTomlInner {
    type Target = BaseCargoToml;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CargoTomlInner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
