// `package.json`

#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::host_log;
use moon_pdk_api::{AnyResult, json_config};
use nodejs_package_json::{PackageJson as BasePackageJson, WorkspacesField};
use starbase_utils::json::JsonValue;

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

        match field {
            "packageManager" => {
                if let Some(pm) = &self.package_manager {
                    root.insert("packageManager".into(), JsonValue::String(pm.into()));
                } else {
                    root.remove("packageManager");
                }
            }
            _ => {}
        };

        Ok(())
    }
}

impl PackageJson {
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

        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Setting <property>packageManager</file> in <path>{}</path>",
                self.path,
            );
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
