use super::parse_version_spec;
use crate::config::CatalogsMap;
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use nodejs_package_json::VersionProtocol;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::fs;
use yarn_lock_parser::parse_str;

pub(crate) fn parse_yarn_lock_content<T: AsRef<str>>(
    content: T,
    output: &mut ParseLockOutput,
) -> AnyResult<()> {
    let lock = parse_str(content.as_ref())?;

    for entry in lock.entries {
        if
        // Root package
        entry.name.contains("root-workspace") ||
            // Workspace package
            entry.resolved.contains("workspace:")
        {
            continue;
        }

        if entry.integrity.is_empty() {
            output
                .dependencies
                .entry(entry.name.to_string())
                .or_default()
                .push(LockDependency {
                    version: parse_version_spec(entry.version)?,
                    ..Default::default()
                });
        } else {
            output
                .dependencies
                .entry(entry.name.to_string())
                .or_default()
                .push(LockDependency {
                    version: parse_version_spec(entry.version)?,
                    hash: Some(entry.integrity.into()),
                    ..Default::default()
                });
        }
    }

    Ok(())
}

pub fn parse_yarn_lock(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;

    parse_yarn_lock_content(content, output)
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct YarnRc {
    pub catalog: Option<FxHashMap<String, VersionProtocol>>,
    pub catalogs: Option<FxHashMap<String, FxHashMap<String, VersionProtocol>>>,
}

impl YarnRc {
    /// Extract all catalogs for the workspace.
    pub fn extract_catalogs(&self) -> Option<CatalogsMap> {
        let mut catalogs = self.catalogs.clone().unwrap_or_default();

        if let Some(data) = self.catalog.clone() {
            catalogs.insert("__default__".into(), data);
        }

        if catalogs.is_empty() {
            return None;
        }

        Some(catalogs)
    }
}
