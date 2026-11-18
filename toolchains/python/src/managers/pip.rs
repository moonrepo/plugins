use super::parse_version_spec;
use crate::pyproject_toml::{PoetryDependency, PyProjectToml, normalize_distribution_name};
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
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

fn create_dep_config_from_requirement(req: &Requirement) -> ManifestDependencyConfig {
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

    dep
}

/// A version that matches the first locked entry of a dependency, so the
/// lockfile decides the version that the task hasher records (D4).
fn any_version() -> UnresolvedVersionSpec {
    // ADR-0005.1
    UnresolvedVersionSpec::parse("*").unwrap()
}

/// moon's task hasher skips a manifest dependency without a version, so one
/// without a version that it can parse, or with a URL, matches any version.
fn insert_manifest_deps(
    deps: &mut BTreeMap<String, ManifestDependency>,
    reqs: impl IntoIterator<Item = Requirement>,
) {
    for req in reqs {
        let mut dep = create_dep_config_from_requirement(&req);

        if dep.version.is_none() {
            dep.version = Some(any_version());
        }

        // A dependency in multiple tables or extras keeps the first
        deps.entry(req.name.to_string())
            .or_insert(ManifestDependency::Config(dep));
    }
}

fn insert_poetry_deps(
    deps: &mut BTreeMap<String, ManifestDependency>,
    table: &BTreeMap<String, PoetryDependency>,
) {
    for (name, dep) in table {
        // The project's Python, not a package
        if name == "python" {
            continue;
        }

        let version = dep
            .get_version()
            .and_then(|version| {
                UnresolvedVersionSpec::parse(version)
                    .inspect_err(|error| debug!("Skipping the version of {name}: {error}"))
                    .ok()
            })
            .unwrap_or_else(any_version);

        // Keys are written as-is, while the lockfile has normalized names
        deps.entry(normalize_distribution_name(name))
            .or_insert(ManifestDependency::Config(ManifestDependencyConfig {
                version: Some(version),
                ..Default::default()
            }));
    }
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
    insert_manifest_deps(
        &mut output.dependencies,
        parse_requirement_lines(path)?
            .into_iter()
            .map(|(req, _)| req),
    );

    Ok(())
}

pub fn parse_requirements_txt(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    for (req, hash) in parse_requirement_lines(path)? {
        let config = create_dep_config_from_requirement(&req);
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

    if let Some(version) = manifest
        .project
        .as_ref()
        .and_then(|project| project.version.as_ref())
    {
        // Normalize the PEP 440 version before converting it to semver
        output.version = pep440_rs::Version::from_str(version)
            .map_err(|error| error.to_string())
            .and_then(|version| {
                Version::parse(version.to_string()).map_err(|error| error.to_string())
            })
            .inspect_err(|error| debug!("Skipping the version in {path}: {error}"))
            .ok();
    }

    // Every declared dependency, so that upgrading any of them in the lockfile
    // changes the hash of tasks (D4)
    insert_manifest_deps(
        &mut output.dependencies,
        manifest
            .get_dependencies()
            .into_iter()
            .chain(manifest.get_optional_dependencies()),
    );
    insert_manifest_deps(
        &mut output.dev_dependencies,
        manifest
            .get_dependency_groups()
            .into_iter()
            .chain(manifest.get_uv_dev_dependencies()),
    );
    // Never locked, but the backend changes what `-e .` installs
    insert_manifest_deps(
        &mut output.build_dependencies,
        manifest.get_build_requirements(),
    );

    if let Some(poetry) = manifest.get_tool_poetry() {
        insert_poetry_deps(&mut output.dependencies, &poetry.dependencies);

        for group in poetry.group.values() {
            insert_poetry_deps(&mut output.dev_dependencies, &group.dependencies);
        }

        insert_poetry_deps(&mut output.dev_dependencies, &poetry.dev_dependencies);
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
