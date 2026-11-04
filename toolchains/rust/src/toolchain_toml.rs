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

        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Setting <property>{field}</property> in <path>{}</path>",
                self.path,
            );
        }

        let (key, value) = match field {
            "toolchain.channel" => match &self.toolchain.channel {
                Some(channel) => ("channel", TomlValue::String(channel.to_owned())),
                None => return Ok(()),
            },
            "toolchain.components" => ("components", to_toml_list(&self.toolchain.components)),
            "toolchain.targets" => ("targets", to_toml_list(&self.toolchain.targets)),
            _ => return Ok(()),
        };

        let toolchain = root
            .entry("toolchain")
            .or_insert_with(|| TomlValue::Table(TomlTable::new()));

        if let Some(inner) = toolchain.as_table_mut() {
            inner.insert(key.into(), value);
        }

        Ok(())
    }
}

fn to_toml_list(list: &[String]) -> TomlValue {
    TomlValue::Array(list.iter().cloned().map(TomlValue::String).collect())
}

// Add missing items, but keep those added manually
fn merge_list(list: &mut Vec<String>, items: &[String]) -> bool {
    let mut changed = false;

    for item in items {
        if !list.contains(item) {
            list.push(item.to_owned());
            changed = true;
        }
    }

    changed
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

        self.toolchain.channel = Some(channel.into());
        self.dirty.push("toolchain.channel".into());

        Ok(true)
    }

    pub fn add_components(&mut self, components: &[String]) -> AnyResult<bool> {
        if !merge_list(&mut self.toolchain.components, components) {
            return Ok(false);
        }

        self.dirty.push("toolchain.components".into());

        Ok(true)
    }

    pub fn add_targets(&mut self, targets: &[String]) -> AnyResult<bool> {
        if !merge_list(&mut self.toolchain.targets, targets) {
            return Ok(false);
        }

        self.dirty.push("toolchain.targets".into());

        Ok(true)
    }
}
