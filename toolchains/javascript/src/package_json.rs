// `package.json`

use crate::config::CatalogsMap;
#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::{HostLogInput, host_log};
use moon_pdk_api::{AnyResult, json_config};
use nodejs_package_json::{PackageJson as BasePackageJson, VersionProtocol, WorkspacesField};
use rustc_hash::FxHashMap;
use starbase_utils::json::{JsonValue, serde_json::json};
use std::collections::BTreeMap;

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

json_config!(PackageJson, BasePackageJson);

impl PackageJson {
    fn save_field(&self, field: &str, config: &mut JsonValue) -> AnyResult<()> {
        let Some(root) = config.as_object_mut() else {
            return Ok(());
        };

        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Setting <property>{field}</file> in <path>{}</path>",
                self.path,
            );
        }

        let mut save_deps =
            |field_name: &str, maybe_deps: Option<&BTreeMap<String, VersionProtocol>>| {
                if let Some(deps) = maybe_deps {
                    let current = root.entry(field_name).or_insert_with(|| json!({}));

                    for (name, version) in deps {
                        current[name] = JsonValue::String(version.to_string());
                    }
                }
            };

        match field {
            "dependencies" => {
                save_deps("dependencies", self.data.dependencies.as_ref());
            }
            "devDependencies" => {
                save_deps("devDependencies", self.data.dev_dependencies.as_ref());
            }
            "packageManager" => {
                if let Some(pm) = &self.package_manager {
                    root.insert("packageManager".into(), JsonValue::String(pm.into()));
                } else {
                    root.remove("packageManager");
                }
            }
            "peerDependencies" => {
                save_deps("peerDependencies", self.data.peer_dependencies.as_ref());
            }
            _ => {}
        };

        Ok(())
    }
}

impl PackageJson {
    /// Add a package and version range to the `dependencies` field.
    pub fn add_dependency<T: AsRef<str>>(
        &mut self,
        name: T,
        range: VersionProtocol,
        if_missing: bool,
    ) -> AnyResult<bool> {
        let name = name.as_ref();

        if internal_add_dependency(name, range, if_missing, &mut self.data.dependencies) {
            self.dirty.push("dependencies".into());

            #[cfg(feature = "wasm")]
            {
                host_log!(
                    "Adding <id>{name}</id> as a production dependency to <path>{}</path>",
                    self.path,
                );
            }

            return Ok(true);
        }

        Ok(false)
    }

    /// Add a package and version range to the `devDependencies` field.
    pub fn add_dev_dependency<T: AsRef<str>>(
        &mut self,
        name: T,
        range: VersionProtocol,
        if_missing: bool,
    ) -> AnyResult<bool> {
        let name = name.as_ref();

        if internal_add_dependency(name, range, if_missing, &mut self.data.dev_dependencies) {
            self.dirty.push("devDependencies".into());

            #[cfg(feature = "wasm")]
            {
                host_log!(
                    "Adding <id>{name}</id> as a development dependency to <path>{}</path>",
                    self.path,
                );
            }

            return Ok(true);
        }

        Ok(false)
    }

    /// Add a package and version range to the `peerDependencies` field.
    pub fn add_peer_dependency<T: AsRef<str>>(
        &mut self,
        name: T,
        range: VersionProtocol,
        if_missing: bool,
    ) -> AnyResult<bool> {
        let name = name.as_ref();

        if internal_add_dependency(name, range, if_missing, &mut self.data.peer_dependencies) {
            self.dirty.push("peerDependencies".into());

            #[cfg(feature = "wasm")]
            {
                host_log!(
                    "Adding <id>{name}</id> as a peer dependency to <path>{}</path>",
                    self.path,
                );
            }

            return Ok(true);
        }

        Ok(false)
    }

    /// Extract all catalogs for the workspace.
    pub fn extract_catalogs(&self) -> Option<CatalogsMap> {
        let mut catalogs: CatalogsMap = FxHashMap::default();

        let mut add_catalog = |name: &str, map: &BTreeMap<String, VersionProtocol>| {
            catalogs
                .entry(name.to_owned())
                .or_default()
                .extend(FxHashMap::from_iter(map.to_owned()));
        };

        // Extract root level first
        if let Some(map) = &self.catalog {
            add_catalog("__default__", map);
        }

        if let Some(data) = &self.catalogs {
            for (name, map) in data {
                add_catalog(name, map);
            }
        }

        // Then extract workspace level
        if let Some(WorkspacesField::Config {
            catalog, catalogs, ..
        }) = &self.workspaces
        {
            if let Some(map) = catalog {
                add_catalog("__default__", map);
            }

            if let Some(data) = catalogs {
                for (name, map) in data {
                    add_catalog(name, map);
                }
            }
        }

        if catalogs.is_empty() {
            return None;
        }

        Some(catalogs)
    }

    /// Extract package members if the current manifest is a workspace.
    pub fn extract_members(&self) -> Option<Vec<String>> {
        self.workspaces.as_ref().map(|ws| match ws {
            WorkspacesField::Globs(globs) => globs.clone(),
            WorkspacesField::Config { packages, .. } => packages.clone(),
        })
    }

    /// Set the `packageManager` field.
    /// Return true if the new value is different from the old value.
    pub fn set_package_manager<T: AsRef<str>>(&mut self, value: T) -> AnyResult<bool> {
        let value = value.as_ref();

        if self
            .data
            .package_manager
            .as_ref()
            .is_some_and(|v| v == value)
        {
            return Ok(false);
        }

        self.dirty.push("packageManager".into());

        if value.is_empty() {
            self.data.package_manager = None;
        } else {
            self.data.package_manager = Some(value.to_owned());
        }

        Ok(true)
    }
}

fn internal_add_dependency(
    name: &str,
    range: VersionProtocol,
    if_missing: bool,
    dependencies: &mut Option<BTreeMap<String, VersionProtocol>>,
) -> bool {
    if name.is_empty() {
        return false;
    }

    // Only add if the dependency doesnt already exist
    if if_missing
        && dependencies
            .as_ref()
            .is_some_and(|map| map.contains_key(name))
    {
        return false;
    }

    dependencies
        .get_or_insert_default()
        .insert(name.to_owned(), range.to_owned());

    true
}
