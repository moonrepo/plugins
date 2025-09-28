use super::parse_version_spec;
use super::yarn::parse_yarn_lock_content;
use moon_pdk::{AnyResult, ExecCommandInput, VirtualPath, exec};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use serde::Deserialize;
use starbase_utils::{fs, json};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BunLockPackageJson {
    pub name: String,
    pub version: Option<String>,
    pub dependencies: BTreeMap<String, String>,
    pub dev_dependencies: BTreeMap<String, String>,
    pub peer_dependencies: BTreeMap<String, String>,
    pub optional_dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BunLockPackage {
    Dependency1(
        String,             // identifier
        String,             // ???
        BunLockPackageJson, // dependencies
        String,             // sha
    ),

    Dependency2(
        String,             // identifier
        BunLockPackageJson, // dependencies
        String,             // sha
    ),

    Dependency3(
        String,             // identifier
        BunLockPackageJson, // dependencies
    ),

    // Must be last!
    #[allow(dead_code)]
    Workspace(Vec<String>),
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BunLock {
    pub lockfile_version: u32,
    pub packages: BTreeMap<String, BunLockPackage>,
    pub patched_dependencies: BTreeMap<String, String>,
    pub overrides: BTreeMap<String, String>,
    pub workspaces: BTreeMap<String, BunLockPackageJson>,
}

pub fn parse_bun_lock(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;
    let lock: BunLock = json::parse(&content)?; // JSON5

    for package in lock.packages.into_values() {
        let (name, version, integrity) = match &package {
            BunLockPackage::Workspace(values) => {
                if let Some((name, suffix)) = values[0].rsplit_once('@')
                    && let Some(ref_name) = suffix.strip_prefix("workspace:")
                    && let Some(ref_package) = lock.workspaces.get(ref_name)
                {
                    output
                        .dependencies
                        .entry(name.to_string())
                        .or_default()
                        .push(LockDependency {
                            version: match &ref_package.version {
                                Some(version) => parse_version_spec(version)?,
                                None => None,
                            },
                            ..Default::default()
                        });
                }

                continue;
            }
            BunLockPackage::Dependency1(id, _unknown, _data, integrity) => {
                let Some((name, version)) = id.rsplit_once('@') else {
                    continue;
                };

                (name, version, Some(integrity))
            }
            BunLockPackage::Dependency2(id, _data, integrity) => {
                let Some((name, version)) = id.rsplit_once('@') else {
                    continue;
                };

                (name, version, Some(integrity))
            }
            BunLockPackage::Dependency3(id, _data) => {
                let Some((name, version)) = id.rsplit_once('@') else {
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
                version: parse_version_spec(version)?,
                hash: integrity.cloned(),
                ..Default::default()
            });
    }

    Ok(())
}

pub fn parse_bun_lockb(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = exec(ExecCommandInput::pipe("bun", ["bun.lockb"]).cwd(path.parent().unwrap()))?;

    parse_yarn_lock_content(content.stdout.trim(), output)
}
