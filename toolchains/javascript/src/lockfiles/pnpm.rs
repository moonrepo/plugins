use super::{parse_name_and_version, parse_version_spec};
use crate::config::CatalogsMap;
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use nodejs_package_json::VersionProtocol;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::{fs, yaml};

pub fn parse_pnpm_lock_yaml(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;

    // pnpm v10 with `managePackageManagerVersions` writes a multi-document
    // pnpm-lock.yaml — the package-manager metadata in one document and the
    // project lockfile in another, separated by `---` markers. Merge packages
    // from every document so we don't fail on `more than one document`.
    let mut packages = FxHashMap::default();
    for doc in yaml::serde_yaml::Deserializer::from_str(&content) {
        let lock = PnpmLock::deserialize(doc)?;
        packages.extend(lock.packages);
    }

    for (name, package) in packages {
        let Some((name, version)) = parse_name_and_version(&name, "(") else {
            continue;
        };

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
            catalogs.insert("__default__".into(), data);
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
