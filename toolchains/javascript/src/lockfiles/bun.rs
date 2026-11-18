use super::yarn::parse_yarn_lock_content;
use super::{parse_name_and_version, parse_version_spec};
#[cfg(feature = "wasm")]
use crate::config::SharedPackageManagerConfig;
#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::{
    ExecCommandInput, HostLogInput, HostLogTarget, exec, host_log, load_toolchain_config,
};
use moon_pdk_api::{AnyResult, LockDependency, ParseLockOutput, VirtualPath};
use serde::Deserialize;
use starbase_utils::{fs, json, json::JsonValue};
use std::collections::BTreeMap;
use toolchain_common::debug;

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

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

// Bun writes `""` when the registry sent no integrity
fn get_integrity(value: &String) -> Option<&String> {
    is_integrity_hash(value).then_some(value)
}

pub fn parse_bun_lock(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    parse_bun_lock_content(&fs::read_file(path)?, output)
}

pub fn parse_bun_lock_content(content: &str, output: &mut ParseLockOutput) -> AnyResult<()> {
    let lock: BunLock = json::parse(content)?; // JSON5

    for (key, package) in lock.packages {
        let (name, version, integrity) = match &package {
            BunLockPackage::Unknown(_) => {
                debug!("Skipping the package {key} in bun.lock, as its entry has an unknown shape");

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

                (name, version, get_integrity(integrity))
            }
            BunLockPackage::Dependency2(id, _data, _tag, integrity) => {
                let Some((name, version)) = parse_name_and_version(id, "") else {
                    continue;
                };

                (name, version, get_integrity(integrity))
            }
            // Tarballs store an integrity hash, while git/github store a bun tag
            BunLockPackage::Dependency3(id, _data, integrity_or_tag) => {
                let Some((name, version)) = parse_name_and_version(id, "") else {
                    continue;
                };

                (name, version, get_integrity(integrity_or_tag))
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
    let mut command = ExecCommandInput::pipe("bun", ["bun.lockb"])
        .cwd(path.parent().expect("file path should have a parent"));

    // Commands run with moon's environment, not the toolchain's, so pin
    // proto's shim to the configured Bun. Repos without the `bun` toolchain
    // can't load its config, so ignore the error
    if let Ok(config) = load_toolchain_config::<SharedPackageManagerConfig>("bun")
        && let Some(version) = config.version
    {
        command
            .env
            .insert("PROTO_BUN_VERSION".into(), version.to_string());
    }

    let error = match exec(command) {
        Ok(result) if result.exit_code == 0 => {
            return parse_yarn_lock_content(result.stdout.trim(), output);
        }
        Ok(result) => result.stderr.trim().to_owned(),
        Err(error) => error.to_string(),
    };

    // ADR-0007.3: The lockfile only refines task hashes, so skip it instead
    // of failing every task, and only warn once, as this runs for every task
    if var::get::<bool>("bun-lockb-warned")?.is_some() {
        debug!("Skipping bun.lockb, as bun couldn't read it: {error}");
    } else {
        host_log!(
            warn,
            "Skipping <file>bun.lockb</file> in task hashes, as <shell>bun bun.lockb</shell> failed: {error}"
        );

        if let Err(error) = var::set("bun-lockb-warned", true) {
            debug!("Failed to cache the bun.lockb warning: {error}");
        }
    }

    Ok(())
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
    fn parses_git_entries() {
        assert!(matches!(
            parse(
                r#"["is-odd@git+https://github.com/jonschlinkert/is-odd.git#4f2a8ac", {}, "jonschlinkert-is-odd-4f2a8ac", "sha512-Ab3d=="]"#
            ),
            BunLockPackage::Dependency2(..)
        ));
        assert!(matches!(
            parse(
                r#"["is-odd@git+https://github.com/jonschlinkert/is-odd.git#4f2a8ac", {}, "jonschlinkert-is-odd-4f2a8ac"]"#
            ),
            BunLockPackage::Dependency3(..)
        ));
    }

    #[test]
    fn parses_tarball_entries() {
        assert!(matches!(
            parse(r#"["mri@https://example.com/mri-1.2.0.tgz", {}, "sha512-Ab3d=="]"#),
            BunLockPackage::Dependency3(..)
        ));
        assert!(matches!(
            parse(r#"["mri@./vendor/mri-1.2.0.tgz", {}, "sha512-Ab3d=="]"#),
            BunLockPackage::Dependency3(..)
        ));
    }

    #[test]
    fn parses_folder_and_root_entries() {
        assert!(matches!(
            parse(r#"["c@file:packages/c", {}]"#),
            BunLockPackage::Dependency4(..)
        ));
        assert!(matches!(
            parse(r#"["root@root:", {}]"#),
            BunLockPackage::Dependency4(..)
        ));
    }

    fn parse_content(content: &str) -> BTreeMap<String, Vec<LockDependency>> {
        let mut output = ParseLockOutput::default();
        parse_bun_lock_content(content, &mut output).unwrap();
        output.dependencies
    }

    #[test]
    fn records_aliases_under_the_real_name() {
        let deps =
            parse_content(r#"{ "packages": { "my-ms": ["ms@2.1.2", "", {}, "sha512-Ab3d=="] } }"#);

        assert_eq!(deps["ms"][0].hash.as_deref(), Some("sha512-Ab3d=="));
        assert!(!deps.contains_key("my-ms"));
    }

    #[test]
    fn skips_integrities_that_arent_hashes() {
        let deps = parse_content(
            r#"{ "packages": {
                "ms": ["ms@2.1.2", "", {}, ""],
                "is-odd": ["is-odd@github:jonschlinkert/is-odd#4f2a8ac", {}, "jonschlinkert-is-odd-4f2a8ac"]
            } }"#,
        );

        assert_eq!(deps["ms"][0].hash, None);
        assert!(deps["ms"][0].version.is_some());
        assert_eq!(deps["is-odd"][0].hash, None);
    }

    #[test]
    fn parses_unknown_entry() {
        assert!(matches!(
            parse(r#"{ "name": "a" }"#),
            BunLockPackage::Unknown(..)
        ));
    }
}
