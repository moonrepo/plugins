use super::{parse_version, parse_version_spec};
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};

pub fn parse_pnpm_lock_yaml(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let lock = chaste_pnpm::parse(path.parent().unwrap())?;
    let root_package = lock.root_package();

    for package in lock.packages() {
        let Some(name) = package.name() else {
            continue;
        };

        if package == root_package {
            continue;
        }

        let mut dep = LockDependency::default();

        if let Some(version) = package.version() {
            dep.version = parse_version_spec(version.to_string())?;
        }

        if let Some(checksum) = package.checksums() {
            let hash = checksum.integrity().to_string();

            if hash != "sha256-" && hash != "sha512-" {
                dep.hash = Some(hash);
            }
        }

        output
            .dependencies
            .entry(name.to_string())
            .or_default()
            .push(dep);
    }

    for package in lock.workspace_members() {
        if let Some(name) = package.name() {
            output.packages.insert(
                name.to_string(),
                match package.version() {
                    Some(version) => parse_version(version.to_string())?,
                    None => None,
                },
            );
        }
    }

    Ok(())
}
