use super::parse_version_spec;
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use package_lock_json_parser::{PackageLockJson, V1Dependency, parse};
use starbase_utils::fs;
use std::collections::HashMap;

fn parse_v1(lock: PackageLockJson, output: &mut ParseLockOutput) -> AnyResult<()> {
    fn add_deps(
        deps: HashMap<String, V1Dependency>,
        output: &mut ParseLockOutput,
    ) -> AnyResult<()> {
        for (name, dep) in deps {
            if let Some(nested_deps) = dep.dependencies {
                add_deps(nested_deps, output)?;
            }

            output
                .dependencies
                .entry(name)
                .or_default()
                .push(LockDependency {
                    version: parse_version_spec(dep.version)?,
                    hash: dep.integrity,
                    ..Default::default()
                });
        }

        Ok(())
    }

    if let Some(dependencies) = lock.dependencies {
        add_deps(dependencies, output)?;
    }

    Ok(())
}

fn parse_v2_up(lock: PackageLockJson, output: &mut ParseLockOutput) -> AnyResult<()> {
    let Some(packages) = lock.packages else {
        return Ok(());
    };

    for (name, package) in packages {
        // Root package
        if name.is_empty() {
            continue;
        }

        let name = name.strip_prefix("workspaces/").unwrap_or(&name);

        output
            .dependencies
            .entry(name.to_string())
            .or_default()
            .push(LockDependency {
                version: parse_version_spec(package.version)?,
                hash: package.integrity,
                ..Default::default()
            });
    }

    Ok(())
}

pub fn parse_package_lock_json(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;
    let lock = parse(&content)?;

    match lock.lockfile_version {
        1 => parse_v1(lock, output)?,
        2 | 3 => parse_v2_up(lock, output)?,
        _ => {}
    };

    Ok(())
}
