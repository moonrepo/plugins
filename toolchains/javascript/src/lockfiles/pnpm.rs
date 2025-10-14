use crate::config::CatalogsMap;
use super::parse_version_spec;
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use nodejs_package_json::VersionProtocol;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::{fs, yaml};

pub fn parse_pnpm_lock_yaml(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;
    let lock: PnpmLock = yaml::parse(&content)?;

    for (name, package) in lock.packages {
        let (name, version) = parse_name_and_version(&name);
        let reso = package.resolution;

        output
            .dependencies
            .entry(name.to_string())
            .or_default()
            .push(LockDependency {
                version: parse_version_spec(version)?,
                hash: reso.integrity.or(reso.commit),
                meta: reso.tarball.or(reso.repo).or(reso.url),
                ..Default::default()
            });
    }

    Ok(())
}

fn parse_name_and_version(value: &str) -> (&str, &str) {
    // Remove parents: @jest/core@29.7.0(@babel/types@7.26.10)
    let value = match value.find('(') {
        Some(index) => &value[0..index],
        None => value,
    };

    // Split on @ but preserve scope: @jest/core@29.7.0
    if let Some(index) = value.rfind('@')
        && index != 0
    {
        return (&value[0..index], &value[index + 1..]);
    }

    // No version? Provide a fake value
    (value, "0.0.0")
}

// pub fn parse_pnpm_lock_yaml(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
//     let lock = chaste_pnpm::parse(path.parent().unwrap())?;
//     let root_package = lock.root_package();

//     for package in lock.packages() {
//         let Some(name) = package.name() else {
//             continue;
//         };

//         if package == root_package {
//             continue;
//         }

//         let mut dep = LockDependency::default();

//         if let Some(version) = package.version() {
//             dep.version = parse_version_spec(version.to_string())?;
//         }

//         if let Some(checksum) = package.checksums() {
//             let hash = checksum.integrity().to_string();

//             if hash != "sha256-" && hash != "sha512-" {
//                 dep.hash = Some(hash);
//             }
//         }

//         output
//             .dependencies
//             .entry(name.to_string())
//             .or_default()
//             .push(dep);
//     }

//     for package in lock.workspace_members() {
//         if let Some(name) = package.name() {
//             output.packages.insert(
//                 name.to_string(),
//                 match package.version() {
//                     Some(version) => parse_version(version.to_string())?,
//                     None => None,
//                 },
//             );
//         }
//     }

//     Ok(())
// }

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PnpmWorkspace {
    pub catalog: Option<FxHashMap<String, VersionProtocol>>,
    pub catalogs: Option<FxHashMap<String, FxHashMap<String, VersionProtocol>>>,
    pub packages: Option<Vec<String>>,
}

impl PnpmWorkspace {
    /// Extract all catalogs for the workspace.
    pub fn extract_catalogs(&self) -> Option<CatalogsMap> {
        let mut catalogs = self.catalogs.clone().unwrap_or_default();

        if let Some(data) = self.catalog.clone() {
            catalogs.insert("default".into(), data);
        }

        if catalogs.is_empty() {
            return None;
        }

        Some(catalogs)
    }
}

// https://github.com/pnpm/pnpm/blob/main/lockfile/types/src/index.ts
// https://github.com/pnpm/pnpm/blob/main/lockfile/types/src/lockfileFileTypes.ts
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PnpmLock {
    pub packages: FxHashMap<String, PnpmLockPackage>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PnpmLockResolution {
    pub integrity: Option<String>,
    // binary
    pub url: Option<String>,
    // git
    pub commit: Option<String>,
    pub repo: Option<String>,
    // tarball
    pub tarball: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PnpmLockPackage {
    pub name: Option<String>,
    pub resolution: PnpmLockResolution,
    pub version: Option<String>,
}
