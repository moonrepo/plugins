use super::parse_version_spec;
use moon_config::UnresolvedVersionSpec;
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use pep508_rs::{Requirement, VerbatimUrl, VersionOrUrl};
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, toml};
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::str::FromStr;

pub fn parse_requirements_txt(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let file = fs::open_file(&path)?;

    for line in io::BufReader::new(file).lines().map_while(Result::ok) {
        if let Ok(parsed) = Requirement::<VerbatimUrl>::from_str(&line) {
            let mut dep = LockDependency::default();

            if let Some(VersionOrUrl::VersionSpecifier(spec)) = parsed.version_or_url {
                dep.req = UnresolvedVersionSpec::parse(spec.to_string()).ok();
            }

            if !parsed.extras.is_empty() {
                dep.meta = Some(
                    parsed
                        .extras
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }

            output
                .dependencies
                .entry(parsed.name.to_string())
                .or_default()
                .push(dep);
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PyLockPackageDist {
    pub url: String,
    pub hashes: HashMap<String, String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PyLockPackage {
    pub archive: Option<PyLockPackageDist>,
    pub name: String,
    pub version: Option<String>,
    pub sdist: Option<PyLockPackageDist>,
}

// https://peps.python.org/pep-0751/
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PyLock {
    pub extras: Vec<String>,
    pub lock_version: Option<String>,
    pub packages: Vec<PyLockPackage>,
    pub requires_python: Option<String>,
}

pub fn parse_pylock_toml(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;
    let lock: PyLock = toml::parse(&content)?;

    for package in lock.packages {
        let mut dep = LockDependency {
            version: match package.version {
                Some(version) => parse_version_spec(version)?,
                None => None,
            },
            ..Default::default()
        };

        if let Some(archive) = package.archive {
            if let Some(hash) = archive.hashes.get("sha256") {
                dep.hash = Some(hash.to_owned());
            }
        } else if let Some(sdist) = package.sdist {
            if let Some(hash) = sdist.hashes.get("sha256") {
                dep.hash = Some(hash.to_owned());
            }
        }

        output
            .dependencies
            .entry(package.name)
            .or_default()
            .push(dep);
    }

    Ok(())
}
