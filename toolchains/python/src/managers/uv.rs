use super::parse_version_spec;
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, toml};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UvLockPackageSdist {
    pub url: String,
    pub hash: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UvLockPackage {
    pub name: String,
    pub version: String,
    pub sdist: UvLockPackageSdist,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UvLock {
    pub package: Vec<UvLockPackage>,
    pub requires_python: Option<String>,
    pub revision: Option<u8>,
    pub version: Option<u8>,
}

pub fn parse_uv_lock(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;
    let lock: UvLock = toml::parse(&content)?;

    for package in lock.package {
        output
            .dependencies
            .entry(package.name)
            .or_default()
            .push(LockDependency {
                version: parse_version_spec(package.version)?,
                hash: Some(package.sdist.hash),
                ..Default::default()
            });
    }

    Ok(())
}
