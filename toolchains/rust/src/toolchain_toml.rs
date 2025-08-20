// `rust-toolchain.toml`

#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::{HostLogInput, host_log};
use moon_pdk_api::{AnyResult, toml_config};
pub use rust_tool::{ToolchainSection, ToolchainToml as BaseToolchainToml};
use starbase_utils::toml::{TomlTable, TomlValue};

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

toml_config!(ToolchainToml, BaseToolchainToml);

impl ToolchainToml {
    pub fn save_field(&self, field: &str, config: &mut TomlValue) -> AnyResult<()> {
        let Some(root) = config.as_table_mut() else {
            return Ok(());
        };

        if field == "toolchain.channel" {
            if let Some(channel) = &self.toolchain.channel {
                let toolchain = root
                    .entry("toolchain")
                    .or_insert_with(|| TomlValue::Table(TomlTable::new()));

                if let Some(inner) = toolchain.as_table_mut() {
                    inner.insert("channel".into(), TomlValue::String(channel.to_owned()));
                }
            }
        };

        Ok(())
    }
}

impl ToolchainToml {
    pub fn set_channel(&mut self, channel: impl AsRef<str>) -> AnyResult<bool> {
        let channel = channel.as_ref();

        if channel.is_empty()
            || self
                .toolchain
                .channel
                .as_ref()
                .is_some_and(|ch| ch == channel)
        {
            return Ok(false);
        }

        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Setting <property>toolchain.channel</file> in <path>{}</path>",
                self.path,
            );
        }

        self.toolchain.channel = Some(channel.into());
        self.dirty.push("toolchain.channel".into());

        Ok(true)
    }
}
