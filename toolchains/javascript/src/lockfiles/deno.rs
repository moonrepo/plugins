use super::{parse_name_and_version, parse_version_spec};
use crate::config::CatalogsMap;
use deno_lockfile::LockfileContent;
use moon_pdk_api::{AnyResult, LockDependency, ParseLockOutput, VirtualPath};
use nodejs_package_json::VersionProtocol;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::json::{self, JsonValue};
use std::collections::BTreeMap;
use toolchain_common::debug;

// Reference: https://github.com/denoland/fresh/blob/main/deno.lock
pub fn parse_deno_lock(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let lockfile_content: JsonValue = json::read_file(path)?;

    // ADR-0008.9: Deno 1.x's v3 nests its packages, which parse as none,
    // and Deno 2 rewrites it on its next install
    if let Some(version) = lockfile_content
        .get("version")
        .and_then(|value| value.as_str())
        && !matches!(version, "4" | "5")
    {
        debug!("Parsing deno.lock version {version} as version 5, which may skip its packages");
    }

    let lockfile = LockfileContent::from_json(lockfile_content)?;

    for (key, value) in lockfile.packages.jsr {
        output
            .dependencies
            .entry(format!("jsr:{}", key.name))
            .or_default()
            .push(LockDependency {
                hash: Some(value.integrity),
                // Version is fully qualified
                version: parse_version_spec(&key.name, key.version.to_string()),
                ..Default::default()
            });
    }

    for (key, value) in lockfile.packages.npm {
        let Some((name, version)) = parse_name_and_version(&key, "_") else {
            continue;
        };

        // Keyed like `package.json` dependencies, while JSR names stay
        // prefixed, as they can overlap npm scopes
        output
            .dependencies
            .entry(name.to_owned())
            .or_default()
            .push(LockDependency {
                hash: value.integrity,
                // Version is fully qualified
                version: parse_version_spec(name, version),
                ..Default::default()
            });
    }

    Ok(())
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct DenoJson {
    pub catalog: Option<FxHashMap<String, VersionProtocol>>,
    pub catalogs: Option<FxHashMap<String, FxHashMap<String, VersionProtocol>>>,
    pub imports: BTreeMap<String, String>,
    pub name: Option<String>,
    pub tasks: BTreeMap<String, DenoJsonTask>,
    #[serde(alias = "workspaces")]
    pub workspace: Option<DenoJsonWorkspace>,
}

impl DenoJson {
    /// Load the `deno.json` or `deno.jsonc` in a directory, with its path.
    pub fn find(root: &VirtualPath) -> AnyResult<Option<(VirtualPath, Self)>> {
        for name in ["deno.json", "deno.jsonc"] {
            let path = root.join(name);

            if path.exists() {
                let config = json::read_file(&path)?;

                return Ok(Some((path, config)));
            }
        }

        Ok(None)
    }

    pub fn load_from(root: &VirtualPath) -> AnyResult<Self> {
        Ok(Self::find(root)?
            .map(|(_, config)| config)
            .unwrap_or_default())
    }

    pub fn extract_catalogs(&self) -> Option<CatalogsMap> {
        let mut catalogs: CatalogsMap = FxHashMap::default();

        let mut add_catalog = |name: &str, map: &FxHashMap<String, VersionProtocol>| {
            let entry = catalogs.entry(name.to_owned()).or_default();

            for (pkg, version) in map {
                entry.insert(pkg.to_owned(), version.to_owned());
            }
        };

        if let Some(map) = &self.catalog {
            add_catalog("__default__", map);
        }

        if let Some(data) = &self.catalogs {
            for (name, map) in data {
                add_catalog(name, map);
            }
        }

        if let Some(DenoJsonWorkspace::Config {
            catalog, catalogs, ..
        }) = &self.workspace
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
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum DenoJsonWorkspace {
    Members(Vec<String>),
    Config {
        #[serde(default)]
        members: Vec<String>,
        #[serde(default)]
        catalog: Option<FxHashMap<String, VersionProtocol>>,
        #[serde(default)]
        catalogs: Option<FxHashMap<String, FxHashMap<String, VersionProtocol>>>,
    },
}

impl DenoJsonWorkspace {
    pub fn get_members(&self) -> &[String] {
        match self {
            Self::Members(members) => members,
            Self::Config { members, .. } => members,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum DenoJsonTask {
    Command(String),
    Config {
        #[serde(default)]
        command: String,
        #[serde(default)]
        dependencies: Vec<String>,
    },
}

impl DenoJsonTask {
    pub fn get_command(&self) -> &String {
        match self {
            Self::Command(command) => command,
            Self::Config { command, .. } => command,
        }
    }

    pub fn get_dependencies(&self) -> &[String] {
        match self {
            Self::Command(_) => &[],
            Self::Config { dependencies, .. } => dependencies,
        }
    }
}

/// The package name of a `jsr:` or `npm:` import, without its version or subpath,
/// like `@std/path` for `jsr:@std/path@^1.0.0/join`.
pub fn get_import_package_name(specifier: &str) -> Option<String> {
    let package = specifier
        .strip_prefix("jsr:")
        .or_else(|| specifier.strip_prefix("npm:"))?;
    let package = package.strip_prefix('/').unwrap_or(package);

    let (scope, rest) = match package.strip_prefix('@') {
        Some(rest) => {
            let (scope, rest) = rest.split_once('/')?;
            (Some(scope), rest)
        }
        None => (None, package),
    };

    let name = &rest[0..rest.find(['@', '/']).unwrap_or(rest.len())];

    if name.is_empty() {
        return None;
    }

    Some(match scope {
        Some(scope) => format!("@{scope}/{name}"),
        None => name.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_import_package_names() {
        for (specifier, name) in [
            ("jsr:@std/path", Some("@std/path")),
            ("jsr:@std/path@^1.0.0", Some("@std/path")),
            ("jsr:@std/path@^1.0.0/join", Some("@std/path")),
            ("jsr:/@std/path@1.0.0", Some("@std/path")),
            ("npm:chalk@^5.3.0", Some("chalk")),
            ("npm:chalk/source", Some("chalk")),
            ("npm:@types/node@22", Some("@types/node")),
            ("npm:@scope", None),
            ("./mod.ts", None),
            ("https://deno.land/x/c/mod.ts", None),
        ] {
            assert_eq!(
                get_import_package_name(specifier).as_deref(),
                name,
                "{specifier}"
            );
        }
    }
}
