use super::parse_version_spec;
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, toml};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PoetryLockPackageFile {
    pub file: String,
    pub hash: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PoetryLockPackage {
    pub files: Vec<PoetryLockPackageFile>,
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PoetryLock {
    pub package: Vec<PoetryLockPackage>,
}

pub fn parse_poetry_lock(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;
    let lock: PoetryLock = toml::parse(&content)?;

    for package in lock.package {
        let mut dep = LockDependency {
            version: parse_version_spec(package.version)?,
            ..Default::default()
        };

        if let Some(file) = package.files.first() {
            dep.hash = Some(file.hash.replace("sha256:", ""));
        }

        output
            .dependencies
            .entry(package.name)
            .or_default()
            .push(dep);
    }

    Ok(())
}
