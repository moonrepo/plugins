use super::yarn::parse_yarn_lock_content;
use super::{parse_name_and_version, parse_version_spec};
#[cfg(feature = "wasm")]
use moon_pdk::{ExecCommandInput, exec};
#[cfg(feature = "wasm")]
use moon_pdk_api::anyhow;
use moon_pdk_api::{AnyResult, LockDependency, ParseLockOutput, VirtualPath};
use serde::Deserialize;
use starbase_utils::{fs, json, json::JsonValue};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BunLockPackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub dependencies: BTreeMap<String, String>,
    pub dev_dependencies: BTreeMap<String, String>,
    pub peer_dependencies: BTreeMap<String, String>,
    pub optional_dependencies: BTreeMap<String, String>,
}

// Entry shapes are defined by bun's lockfile serializer:
// https://github.com/oven-sh/bun/blob/main/src/install/lockfile/bun.lock.rs
//   npm       -> [ "name@version", registry, INFO, integrity ]
//   git       -> [ "name@git+repo", INFO, bun tag, integrity? ]
//   github    -> [ "name@github:user/repo", INFO, bun tag, integrity? ]
//   tarball   -> [ "name@url", INFO, integrity? ]
//   symlink   -> [ "name@link:path", INFO ]
//   folder    -> [ "name@file:path", INFO ]
//   root      -> [ "name@root:", INFO ]
//   workspace -> [ "name@workspace:path" ]
// Bun v1.4 records an integrity hash for git, github, and tarball entries,
// which older lockfiles are missing.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BunLockPackage {
    // npm
    Dependency1(
        String,    // identifier
        String,    // registry
        JsonValue, // object
        String,    // sha
    ),

    // git/github with integrity
    Dependency2(
        String,    // identifier
        JsonValue, // object
        String,    // bun tag
        String,    // sha
    ),

    // git/github without integrity, tarball
    Dependency3(
        String,    // identifier
        JsonValue, // object
        String,    // bun tag or sha
    ),

    // symlink, folder, root
    Dependency4(
        String,    // identifier
        JsonValue, // object
    ),

    // Must be last, other than `Unknown`!
    #[allow(dead_code)]
    Workspace(Vec<String>),

    // ADR-0008.4: Shapes from newer Bun versions, which are skipped
    #[allow(dead_code)]
    Unknown(JsonValue),
}

// Only fields that are actually used are defined, as other fields
// (like `overrides`) change shape between lockfile versions
#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BunLock {
    pub packages: BTreeMap<String, BunLockPackage>,
    pub workspaces: BTreeMap<String, BunLockPackageJson>,
}

// Integrity hashes are SRI formatted: <algorithm>-<base64>
fn is_integrity_hash(value: &str) -> bool {
    ["sha1-", "sha256-", "sha384-", "sha512-"]
        .iter()
        .any(|algorithm| value.starts_with(algorithm))
}

pub fn parse_bun_lock(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;
    let lock: BunLock = json::parse(&content)?; // JSON5

    for (key, package) in lock.packages {
        let (name, version, integrity) = match &package {
            BunLockPackage::Unknown(_) => {
                tracing::debug!(
                    "Skipping the package {key} in bun.lock, as its entry has an unknown shape"
                );

                continue;
            }
            BunLockPackage::Workspace(values) => {
                if let Some((name, ref_name)) = values
                    .first()
                    .and_then(|value| value.split_once("@workspace:"))
                    && let Some(ref_package) = lock.workspaces.get(ref_name)
                {
                    output
                        .dependencies
                        .entry(name.to_string())
                        .or_default()
                        .push(LockDependency {
                            version: ref_package
                                .version
                                .as_ref()
                                .and_then(|version| parse_version_spec(name, version)),
                            ..Default::default()
                        });
                }

                continue;
            }
            BunLockPackage::Dependency1(id, _registry, _data, integrity) => {
                let Some((name, version)) = parse_name_and_version(id, "") else {
                    continue;
                };

                (name, version, Some(integrity))
            }
            BunLockPackage::Dependency2(id, _data, _tag, integrity) => {
                let Some((name, version)) = parse_name_and_version(id, "") else {
                    continue;
                };

                (name, version, Some(integrity))
            }
            // Tarballs store an integrity hash, while git/github store a bun tag
            BunLockPackage::Dependency3(id, _data, integrity_or_tag) => {
                let Some((name, version)) = parse_name_and_version(id, "") else {
                    continue;
                };

                (
                    name,
                    version,
                    if is_integrity_hash(integrity_or_tag) {
                        Some(integrity_or_tag)
                    } else {
                        None
                    },
                )
            }
            BunLockPackage::Dependency4(id, _data) => {
                let Some((name, version)) = parse_name_and_version(id, "") else {
                    continue;
                };

                (name, version, None)
            }
        };

        output
            .dependencies
            .entry(name.to_string())
            .or_default()
            .push(LockDependency {
                version: parse_version_spec(name, version),
                hash: integrity.cloned(),
                ..Default::default()
            });
    }

    Ok(())
}

#[cfg(feature = "wasm")]
pub fn parse_bun_lockb(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = exec(
        ExecCommandInput::pipe("bun", ["bun.lockb"])
            .cwd(path.parent().expect("file path should have a parent")),
    )?;

    if content.exit_code != 0 {
        return Err(anyhow!(
            "Failed to read <file>bun.lockb</file> with <shell>bun</shell>: {}",
            content.stderr.trim()
        ));
    }

    parse_yarn_lock_content(content.stdout.trim(), output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(content: &str) -> BunLockPackage {
        json::parse(content).unwrap()
    }

    #[test]
    fn parses_npm_entry() {
        assert!(matches!(
            parse(r#"["csstype@3.1.3", "", {}, "sha512-M1uQ=="]"#),
            BunLockPackage::Dependency1(..)
        ));
    }

    // https://github.com/moonrepo/plugins/issues/174
    #[test]
    fn parses_github_entry_with_integrity() {
        assert!(matches!(
            parse(
                r#"["@portkey-ai/gateway@github:Portkey-AI/gateway#ca77129", { "dependencies": { "async-retry": "^1.3.3" }, "bin": "build/start-server.js" }, "Portkey-AI-gateway-ca77129", "sha512-71lq=="]"#
            ),
            BunLockPackage::Dependency2(..)
        ));
    }

    // https://github.com/moonrepo/moon/issues/2049
    #[test]
    fn parses_github_entry_without_integrity() {
        assert!(matches!(
            parse(
                r#"["uWebSockets.js@github:uNetworking/uWebSockets.js#6609a88", {}, "uNetworking-uWebSockets.js-6609a88"]"#
            ),
            BunLockPackage::Dependency3(..)
        ));
    }

    #[test]
    fn parses_symlink_entry() {
        assert!(matches!(
            parse(r#"["local-lib@link:packages/local-lib", {}]"#),
            BunLockPackage::Dependency4(..)
        ));
    }

    #[test]
    fn parses_workspace_entry() {
        assert!(matches!(
            parse(r#"["a@workspace:packages/a"]"#),
            BunLockPackage::Workspace(..)
        ));
    }

    #[test]
    fn parses_unknown_entry() {
        assert!(matches!(
            parse(r#"{ "name": "a" }"#),
            BunLockPackage::Unknown(..)
        ));
    }
}
