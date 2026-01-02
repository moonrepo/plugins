use super::parse_version_spec;
use crate::pyproject_toml::PyProjectToml;
use moon_config::{UnresolvedVersionSpec, Version};
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{
    LockDependency, ManifestDependency, ManifestDependencyConfig, ParseLockOutput,
    ParseManifestOutput,
};
use pep440_rs::Operator;
use pep508_rs::{Requirement, VerbatimUrl, VersionOrUrl};
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, toml};
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::str::FromStr;

fn create_manifest_dep_from_requirement(req: &Requirement) -> ManifestDependency {
    let mut dep = ManifestDependencyConfig::default();

    if let Some(version_or_url) = &req.version_or_url {
        match version_or_url {
            VersionOrUrl::Url(url) => {
                dep.url = Some(url.to_string());
            }
            VersionOrUrl::VersionSpecifier(specs) => {
                // Explicit version
                if specs.len() == 1
                    && let Some(spec) = specs.first()
                    && (spec.operator() == &Operator::Equal
                        || spec.operator() == &Operator::ExactEqual)
                {
                    dep.version = UnresolvedVersionSpec::parse(spec.version().to_string()).ok();
                }
                // Version range
                else {
                    dep.version = UnresolvedVersionSpec::parse(specs.to_string()).ok();
                }
            }
        };
    }

    if !req.extras.is_empty() {
        dep.features = req.extras.iter().map(|e| e.to_string()).collect::<Vec<_>>();
    }

    ManifestDependency::Config(dep)
}

pub fn parse_requirements_txt(
    path: &VirtualPath,
    output: &mut ParseManifestOutput,
) -> AnyResult<()> {
    let file = fs::open_file(path)?;

    for line in io::BufReader::new(file).lines().map_while(Result::ok) {
        if let Ok(req) = Requirement::<VerbatimUrl>::from_str(&line) {
            output.dependencies.insert(
                req.name.to_string(),
                create_manifest_dep_from_requirement(&req),
            );
        }
    }

    Ok(())
}

pub fn parse_pyproject_toml(path: &VirtualPath, output: &mut ParseManifestOutput) -> AnyResult<()> {
    let manifest = PyProjectToml::load(path.to_owned())?;

    let Some(project) = &manifest.project else {
        return Ok(());
    };

    if let Some(version) = &project.version {
        output.version = Version::parse(&version.to_string()).ok();
    }

    if let Some(dependencies) = &project.dependencies {
        for req in dependencies {
            output.dependencies.insert(
                req.name.to_string(),
                create_manifest_dep_from_requirement(req),
            );
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
    pub sdist: Option<PyLockPackageDist>,
    pub version: Option<String>,
    pub wheels: Option<Vec<PyLockPackageDist>>,
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
        } else if let Some(wheels) = package.wheels {
            for wheel in wheels {
                if let Some(hash) = wheel.hashes.get("sha256") {
                    dep.hash = Some(hash.to_owned());
                    break;
                }
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
