use super::{parse_name_and_version, parse_version_spec};
use crate::config::CatalogsMap;
use moon_pdk_api::{AnyResult, LockDependency, ParseLockOutput, VirtualPath};
use nodejs_package_json::VersionProtocol;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::fs;
use starbase_utils::json::{self, JsonValue};
use toolchain_common::debug;
use yarn_lock_parser::parse_str;

pub(crate) fn parse_yarn_lock_content<T: AsRef<str>>(
    content: T,
    output: &mut ParseLockOutput,
) -> AnyResult<()> {
    let content = content.as_ref();

    // Before the first install
    if content.trim().is_empty() {
        return Ok(());
    }

    // Yarn 6 writes JSON
    if content.trim_start().starts_with('{') {
        return parse_yarn_lock_json(content, output);
    }

    let lock = parse_str(content)?;

    for entry in lock.entries {
        // The root and workspace packages
        if entry.resolved.contains("workspace:") {
            continue;
        }

        // Berry merges an alias into the entry of the package it points at,
        // like `alias@npm:foo@1.0.0, foo@npm:^1.0.0`, and the entry's name is
        // the first descriptor's, so record it under every name
        let mut names = vec![entry.name];

        for (name, _) in &entry.descriptors {
            if !names.contains(name) {
                names.push(name);
            }
        }

        for name in names {
            output
                .dependencies
                .entry(name.to_string())
                .or_default()
                .push(LockDependency {
                    version: parse_version_spec(name, entry.version),
                    hash: (!entry.integrity.is_empty()).then(|| entry.integrity.into()),
                    ..Default::default()
                });
        }
    }

    Ok(())
}

// ADR-0008.2: Only the fields we read
#[derive(Default, Deserialize)]
#[serde(default)]
struct YarnLockJson {
    entries: FxHashMap<String, JsonValue>,
    islands: FxHashMap<String, FxHashMap<String, JsonValue>>,
}

#[derive(Deserialize)]
struct YarnLockJsonEntry {
    checksum: Option<String>,
    resolution: YarnLockJsonResolution,
}

#[derive(Deserialize)]
struct YarnLockJsonResolution {
    resolution: String,
    version: String,
}

fn parse_yarn_lock_json(content: &str, output: &mut ParseLockOutput) -> AnyResult<()> {
    let lock: YarnLockJson = json::parse(content)?;

    for (key, value) in lock
        .entries
        .into_iter()
        .chain(lock.islands.into_values().flatten())
    {
        // ADR-0008.5
        let entry: YarnLockJsonEntry = match json::serde_json::from_value(value) {
            Ok(entry) => entry,
            Err(error) => {
                debug!("Skipping the yarn.lock entry {key}: {error}");
                continue;
            }
        };

        // The key's descriptors can start with an alias, so take the name
        // from the resolved locator, like `is-number@npm:7.0.0`
        let Some((name, reference)) = parse_name_and_version(&entry.resolution.resolution, "")
        else {
            continue;
        };

        // The root and workspace packages
        if reference.starts_with("workspace:") {
            continue;
        }

        output
            .dependencies
            .entry(name.to_string())
            .or_default()
            .push(LockDependency {
                version: parse_version_spec(name, &entry.resolution.version),
                hash: entry.checksum,
                ..Default::default()
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use moon_pdk_api::VersionSpec;

    fn parse(content: &str) -> ParseLockOutput {
        let mut output = ParseLockOutput::default();
        parse_yarn_lock_content(content, &mut output).unwrap();
        output
    }

    fn versions(output: &ParseLockOutput, name: &str) -> Vec<String> {
        output.dependencies[name]
            .iter()
            .map(|dep| dep.version.as_ref().unwrap().to_string())
            .collect()
    }

    #[test]
    fn parses_an_empty_file() {
        assert!(parse("").dependencies.is_empty());
        assert!(parse("\n").dependencies.is_empty());
    }

    #[test]
    fn records_merged_aliases_under_every_name() {
        let output = parse(
            r#"# This file is generated by running "yarn install" inside your project.

__metadata:
  version: 8
  cacheKey: 10c0

"alias-num@npm:is-number@7.0.0, is-number@npm:7.0.0, is-number@npm:^7.0.0":
  version: 7.0.0
  resolution: "is-number@npm:7.0.0"
  checksum: 10c0/abc
  languageName: node
  linkType: hard

"root-workspace-0b6124@workspace:.":
  version: 0.0.0-use.local
  resolution: "root-workspace-0b6124@workspace:."
  languageName: unknown
  linkType: soft
"#,
        );

        assert_eq!(versions(&output, "is-number"), ["7.0.0"]);
        assert_eq!(versions(&output, "alias-num"), ["7.0.0"]);
        assert_eq!(
            output.dependencies["is-number"][0].hash.as_deref(),
            Some("10c0/abc")
        );
        assert_eq!(output.dependencies.len(), 2);
    }

    #[test]
    fn parses_yarn_6_json() {
        let output = parse(
            r#"{
  "__metadata": { "version": 10 },
  "entries": {
    "a@workspace:packages/a": {
      "checksum": null,
      "resolution": { "resolution": "a@workspace:packages/a", "version": "0.0.0" }
    },
    "root-workspace-0b6124@workspace:.": {
      "checksum": null,
      "resolution": { "resolution": "root-workspace-0b6124@workspace:.", "version": "0.0.0" }
    },
    "alias-num@npm:is-number@7.0.0, is-number@npm:^7.0.0": {
      "checksum": "10c0/abc",
      "resolution": {
        "resolution": "is-number@npm:7.0.0",
        "version": "7.0.0",
        "dependencies": {}
      }
    },
    "@scope/pkg@npm:^1.0.0": {
      "checksum": null,
      "resolution": { "resolution": "@scope/pkg@npm:1.2.0", "version": "1.2.0" }
    },
    "unknown@npm:^1.0.0": { "checksum": 123 }
  },
  "islands": {
    "island-1": {
      "is-odd@npm:^3.0.0": {
        "checksum": "10c0/def",
        "resolution": { "resolution": "is-odd@npm:3.0.1", "version": "3.0.1" }
      }
    }
  }
}"#,
        );

        assert_eq!(output.dependencies.len(), 3);
        assert_eq!(versions(&output, "is-number"), ["7.0.0"]);
        assert_eq!(
            output.dependencies["is-number"][0].hash.as_deref(),
            Some("10c0/abc")
        );
        assert_eq!(
            output.dependencies["@scope/pkg"][0].version,
            Some(VersionSpec::parse("1.2.0").unwrap())
        );
        assert!(output.dependencies["@scope/pkg"][0].hash.is_none());
        assert_eq!(versions(&output, "is-odd"), ["3.0.1"]);
    }
}
