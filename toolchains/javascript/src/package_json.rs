// `package.json`

use crate::config::CatalogsMap;
#[cfg(feature = "wasm")]
use extism_pdk::*;
use lang_javascript_common::insert_dev_engine_version;
#[cfg(feature = "wasm")]
use moon_pdk::{HostLogInput, host_log};
use moon_pdk_api::{AnyResult, json_config};
use nodejs_package_json::{
    PackageJson as BasePackageJson, ScriptsMap, VersionProtocol, WorkspacesField,
};
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
        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Setting <property>{field}</property> in <path>{}</path>",
                self.path,
            );
        }

        // `devEngines.packageManager` is marked dirty as `devEngines:<name>@<version>`,
        // and only its version is replaced, keeping fields like `onFail`
        if let Some(entry) = field.strip_prefix("devEngines:")
            && let Some((name, version)) = entry.rsplit_once('@')
        {
            return insert_dev_engine_version(
                config,
                "packageManager".into(),
                name.into(),
                version.into(),
            );
        }

        let Some(root) = config.as_object_mut() else {
            return Ok(());
        };

        // Dependencies are marked dirty one at a time (`dependencies:<name>`), so
        // that entries we didn't change keep their original text. Re-formatting
        // them could change a range, like `>=1 <2` to `>=1, <2`, which npm rejects.
        let (field, dep_name) = match field.split_once(':') {
            Some((field, name)) => (field, Some(name)),
            None => (field, None),
        };

        let mut save_deps =
            |field_name: &str, maybe_deps: Option<&BTreeMap<String, VersionProtocol>>| {
                if let Some(deps) = maybe_deps
                    && let Some(name) = dep_name
                    && let Some(version) = deps.get(name)
                {
                    let current = root.entry(field_name).or_insert_with(|| json!({}));

                    current[name] = JsonValue::String(version.to_string());
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
            self.dirty.push(format!("dependencies:{name}"));

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
            self.dirty.push(format!("devDependencies:{name}"));

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
            self.dirty.push(format!("peerDependencies:{name}"));

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

    /// The package manager names from `packageManager`, then the first
    /// `devEngines.packageManager` entry, which `pnpm init` (v11+) and nub write.
    pub fn get_declared_package_managers(&self) -> Vec<&str> {
        self.data
            .package_manager
            .as_ref()
            .map(|pm| pm.split_once('@').map(|parts| parts.0).unwrap_or(pm))
            .into_iter()
            .chain(
                self.data
                    .dev_engines
                    .as_ref()
                    .and_then(|engines| engines.package_manager.as_ref())
                    .and_then(|engines| engines.list().first().map(|engine| engine.name.as_str())),
            )
            .collect()
    }

    /// Set the version of the package manager's entry in `devEngines.packageManager`.
    /// Return false when it has no entry for the package manager.
    pub fn set_dev_engine_package_manager(&mut self, name: &str, version: &str) -> bool {
        let Some(item) = self
            .data
            .dev_engines
            .as_ref()
            .and_then(|engines| engines.package_manager.as_ref())
            .and_then(|engine| engine.list().into_iter().find(|item| item.name == name))
        else {
            return false;
        };

        if item
            .version
            .as_ref()
            .is_none_or(|current| current.to_string() != version)
        {
            self.dirty.push(format!("devEngines:{name}@{version}"));
        }

        true
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

    if let Some(current) = dependencies.as_ref().and_then(|map| map.get(name)) {
        // Only add if the dependency doesnt already exist
        if if_missing || *current == range {
            return false;
        }
    }

    dependencies
        .get_or_insert_default()
        .insert(name.to_owned(), range.to_owned());

    true
}

/// Scripts that package managers run when installing a package.
const INSTALL_SCRIPT_NAMES: [&str; 6] = [
    "preinstall",
    "install",
    "postinstall",
    "preprepare",
    "prepare",
    "postprepare",
];

/// The arguments of a package's install scripts that may be files it runs, like
/// `scripts/setup.js` in `node scripts/setup.js && husky`. They aren't checked
/// to exist, so bin names like `husky` are returned too.
pub fn get_install_script_args(scripts: &ScriptsMap) -> Vec<&str> {
    INSTALL_SCRIPT_NAMES
        .iter()
        .filter_map(|name| scripts.get(*name))
        .flat_map(|script| {
            script.split(|ch: char| ch.is_whitespace() || matches!(ch, ';' | '|' | '&' | '(' | ')'))
        })
        .map(|arg| arg.trim_matches(['"', '\'']))
        .filter(|arg| !arg.is_empty() && !arg.starts_with('-'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_install_script_args() {
        let scripts = ScriptsMap::from_iter([
            ("build".into(), "node build.js".into()),
            (
                "postinstall".into(),
                "node --experimental-strip-types scripts/setup.ts && husky".into(),
            ),
            (
                "prepare".into(),
                "node 'tools/prepare.mjs';patch-package".into(),
            ),
        ]);

        assert_eq!(
            get_install_script_args(&scripts),
            [
                "node",
                "scripts/setup.ts",
                "husky",
                "node",
                "tools/prepare.mjs",
                "patch-package"
            ]
        );
    }
}
