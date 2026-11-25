use super::{parse_name_and_version, parse_version_spec};
use crate::config::CatalogsMap;
use moon_pdk_api::{AnyResult, LockDependency, ParseLockOutput, VirtualPath};
use nodejs_package_json::VersionProtocol;
use rustc_hash::FxHashMap;
use serde::{Deserialize, de::IgnoredAny};
use starbase_utils::{fs, yaml};

pub fn parse_pnpm_lock_yaml(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let content = fs::read_file(path)?;

    // pnpm v10 with `managePackageManagerVersions` writes a multi-document
    // pnpm-lock.yaml — the package-manager metadata in one document and the
    // project lockfile in another, separated by `---` markers. Merge packages
    // from every document so we don't fail on `more than one document`.
    let mut packages = FxHashMap::default();
    let mut patches = FxHashMap::default();

    for doc in yaml::serde_yaml::Deserializer::from_str(&content) {
        let lock = PnpmLock::deserialize(doc)?;

        // v9 records patches in `snapshots` keys, and v6 in `packages` keys
        for key in lock.snapshots.keys().chain(lock.packages.keys()) {
            if let Some((package, hash)) = extract_patch_hash(key) {
                patches.insert(package.to_owned(), hash.to_owned());
            }
        }

        packages.extend(lock.packages);
    }

    for (key, package) in packages {
        let key = strip_v6_prefix(&key);

        let Some((name, version)) = parse_name_and_version(key, "(") else {
            continue;
        };

        let reso = package.resolution;
        let mut hash = reso
            .integrity
            .or(package.integrity) // v13
            .or(reso.commit);

        // A patch doesn't change the package's integrity
        if let Some(patch) = patches.get(&format!("{name}@{version}")) {
            hash = Some(match hash {
                Some(hash) => format!("{hash}+patch.{patch}"),
                None => format!("patch.{patch}"),
            });
        }

        output
            .dependencies
            .entry(name.to_string())
            .or_default()
            .push(LockDependency {
                version: parse_version_spec(name, version),
                hash,
                meta: reso.tarball.or(reso.repo).or(reso.url),
                ..Default::default()
            });
    }

    Ok(())
}

// v6 (pnpm 8) keys start with `/`, like `/@babel/core@7.22.5`
fn strip_v6_prefix(key: &str) -> &str {
    key.strip_prefix('/').unwrap_or(key)
}

/// The package and patch hash of a lockfile key, like `is-number@7.0.0`
/// and `0a2b` for `is-number@7.0.0(patch_hash=0a2b)(react@18.3.1)`.
fn extract_patch_hash(key: &str) -> Option<(&str, &str)> {
    let key = strip_v6_prefix(key);
    let (_, rest) = key.split_once("patch_hash=")?;
    let hash = &rest[0..rest.find(')').unwrap_or(rest.len())];
    let package = &key[0..key.find('(')?];

    (!hash.is_empty()).then_some((package, hash))
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

        // The default catalog can also be named `default`, but not both
        if let Some(data) = self.catalog.clone().or_else(|| catalogs.remove("default")) {
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
    pub snapshots: FxHashMap<String, IgnoredAny>,
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
    // v13 (pnpm 12's experimental `lockfile.format`)
    pub integrity: Option<String>,
    pub name: Option<String>,
    pub resolution: PnpmLockResolution,
    pub version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_patch_hashes() {
        assert_eq!(
            extract_patch_hash("is-number@7.0.0(patch_hash=0a2b)"),
            Some(("is-number@7.0.0", "0a2b"))
        );
        assert_eq!(
            extract_patch_hash("/@s/a@1.0.0(patch_hash=0a2b)(react@18.3.1)"),
            Some(("@s/a@1.0.0", "0a2b"))
        );
        assert_eq!(extract_patch_hash("react-dom@18.3.1(react@18.3.1)"), None);
        assert_eq!(extract_patch_hash("react@18.3.1"), None);
    }

    #[test]
    fn maps_the_default_catalog() {
        let workspace: PnpmWorkspace =
            yaml::serde_yaml::from_str("catalogs:\n  default:\n    react: ^18.0.0\n").unwrap();
        let catalogs = workspace.extract_catalogs().unwrap();

        assert!(catalogs.contains_key("__default__"));
        assert!(!catalogs.contains_key("default"));
    }
}
