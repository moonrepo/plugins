use super::parse_version_spec;
use crate::pyproject_toml::PyProjectToml;
use crate::requirements::parse_requirements_file;
use extism_pdk::debug;
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
                    dep.version = UnresolvedVersionSpec::parse(spec.version().to_string())
                        .inspect_err(|error| {
                            debug!("Skipping the version of {}: {error}", req.name)
                        })
                        .ok();
                }
                // Version range
                else {
                    dep.version = UnresolvedVersionSpec::parse(specs.to_string())
                        .inspect_err(|error| {
                            debug!("Skipping the version of {}: {error}", req.name)
                        })
                        .ok();
                }
            }
        };
    }

    if !req.extras.is_empty() {
        dep.features = req.extras.iter().map(|e| e.to_string()).collect::<Vec<_>>();
    }

    ManifestDependency::Config(dep)
}

// ADR-0008.5: Skip a requirement that can't be parsed, instead of failing the whole file
fn parse_requirement_lines(path: &VirtualPath) -> AnyResult<Vec<(Requirement, Option<String>)>> {
    let content = fs::read_file(path)?;

    Ok(parse_requirements_file(&content)
        .into_iter()
        .filter_map(|line| {
            Requirement::<VerbatimUrl>::from_str(&line.requirement)
                .inspect_err(|error| {
                    debug!(
                        "Skipping the requirement {} in {path}: {error}",
                        line.requirement
                    )
                })
                .ok()
                .map(|req| (req, line.hash))
        })
        .collect())
}

pub fn parse_requirements_in(
    path: &VirtualPath,
    output: &mut ParseManifestOutput,
) -> AnyResult<()> {
    for (req, _) in parse_requirement_lines(path)? {
        output.dependencies.insert(
            req.name.to_string(),
            create_manifest_dep_from_requirement(&req),
        );
    }

    Ok(())
}

pub fn parse_requirements_txt(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    for (req, hash) in parse_requirement_lines(path)? {
        let ManifestDependency::Config(config) = create_manifest_dep_from_requirement(&req) else {
            continue;
        };

        let mut dep = LockDependency {
            hash,
            ..Default::default()
        };

        if !config.features.is_empty() {
            dep.meta = Some(config.features.join(","))
        }

        if let Some(version) = config.version {
            if matches!(version, UnresolvedVersionSpec::Version(_)) {
                dep.version = Some(version.to_resolved_spec());
            } else {
                dep.req = Some(version);
            }
        }

        output
            .dependencies
            .entry(req.name.to_string())
            .or_default()
            .push(dep);
    }

    Ok(())
}

pub fn parse_pyproject_toml(path: &VirtualPath, output: &mut ParseManifestOutput) -> AnyResult<()> {
    let manifest = PyProjectToml::load(path.to_owned())?;

    let Some(project) = &manifest.project else {
        return Ok(());
    };

    if let Some(version) = &project.version {
        // Normalize the PEP 440 version before converting it to semver
        output.version = pep440_rs::Version::from_str(version)
            .map_err(|error| error.to_string())
            .and_then(|version| {
                Version::parse(version.to_string()).map_err(|error| error.to_string())
            })
            .inspect_err(|error| debug!("Skipping the version in {path}: {error}"))
            .ok();
    }

    for req in manifest.get_dependencies() {
        output.dependencies.insert(
            req.name.to_string(),
            create_manifest_dep_from_requirement(&req),
        );
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
            version: package
                .version
                .and_then(|version| parse_version_spec(&package.name, version)),
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
