// `Cargo.toml`

pub use cargo_toml::{Inheritable, Manifest as BaseCargoToml};
#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::{HostLogInput, host_log};
use moon_pdk_api::{AnyResult, toml_config};
use serde::{Deserialize, Serialize};
pub use starbase_utils::toml::{self, TomlTable, TomlValue};

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

toml_config!(CargoToml, CargoTomlInner);

fn new_table() -> TomlValue {
    TomlValue::Table(TomlTable::default())
}

impl CargoToml {
    pub fn save_field(&self, field: &str, config: &mut TomlValue) -> AnyResult<()> {
        let Some(root) = config.as_table_mut() else {
            return Ok(());
        };

        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Setting <property>{field}</file> in <path>{}</path>",
                self.path,
            );
        }

        match field {
            "workspace.package.rust-version" => {
                if let Some(version) = self
                    .workspace
                    .as_ref()
                    .and_then(|workspace| workspace.package.as_ref())
                    .and_then(|package| package.rust_version.as_ref())
                {
                    let workspace = root.entry("workspace").or_insert_with(new_table);
                    let package = workspace
                        .as_table_mut()
                        .unwrap()
                        .entry("package")
                        .or_insert_with(new_table);

                    if let Some(inner) = package.as_table_mut() {
                        inner.insert("rust-version".into(), TomlValue::String(version.into()));
                    }
                }
            }

            "package.rust-version" => {
                if let Some(version) = self
                    .package
                    .as_ref()
                    .and_then(|package| package.rust_version())
                {
                    let package = root.entry("package").or_insert_with(new_table);

                    if let Some(inner) = package.as_table_mut() {
                        inner.insert("rust-version".into(), TomlValue::String(version.into()));
                    }
                }
            }
            _ => {}
        };

        Ok(())
    }

    /// Extract package members if the current manifest is a workspace.
    pub fn extract_members(&self) -> Option<Vec<String>> {
        if let Some(workspace) = &self.workspace {
            let mut list = workspace.members.clone();

            // Requires negated globs to exclude
            for ex in &workspace.exclude {
                list.push(format!("!{ex}"));
            }

            Some(list)
        } else {
            None
        }
    }
}

impl CargoToml {
    /// Set the minimum supported rust version (MSRV). If the manifest is a workspace,
    /// update `workspace.package.rust-version`, otherwise update `package.rust-version`.
    pub fn set_msrv(&mut self, version: impl AsRef<str>) -> AnyResult<bool> {
        let version = version.as_ref();
        let mut dirty = None;

        if version.is_empty() {
            return Ok(false);
        }

        if let Some(workspace) = &mut self.workspace {
            let package = workspace.package.get_or_insert_default();

            if package.rust_version.is_none()
                || package
                    .rust_version
                    .as_ref()
                    .is_some_and(|rv| version != rv)
            {
                package.rust_version = Some(version.into());
                dirty = Some("workspace.package.rust-version");
            }
        }

        if let Some(package) = &mut self.package
            && (package.rust_version.is_none()
                || package.rust_version.as_ref().is_some_and(|rv| match rv {
                    Inheritable::Set(inner) => version != inner,
                    Inheritable::Inherited => false,
                }))
        {
            package.set_rust_version(Some(version.into()));
            dirty = Some("package.rust-version");
        }

        if let Some(field) = dirty {
            self.dirty.push(field.into());

            return Ok(true);
        }

        Ok(false)
    }
}

// `cargo_toml::Manifest` does not implement `Default`,
// so we have this hacky workaround...
#[derive(Deserialize, Serialize)]
pub struct CargoTomlInner(BaseCargoToml);

impl CargoTomlInner {
    pub fn new_package() -> Self {
        Self(toml::parse("[package]\nname = \"\"").unwrap())
    }

    pub fn new_workspace() -> Self {
        Self(toml::parse("[workspace]\nresolver = \"2\"").unwrap())
    }
}

impl Default for CargoTomlInner {
    fn default() -> Self {
        Self::new_package()
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
