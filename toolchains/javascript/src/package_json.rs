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
    fn save_field(&self, _field: &str, config: &mut JsonValue) -> AnyResult<()> {
        let Some(_root) = config.as_object_mut() else {
            return Ok(());
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
}
