// Nub installs in the format of the package manager that owns the repo

use crate::config::JavaScriptPackageManager;
use crate::package_json::PackageJson;
use moon_pdk_api::{AnyResult, VirtualPath};
use std::str::FromStr;

// Nub natively uses `nub.lock`, but respects the lockfiles of other
// package managers (for both resolution and layout), so treat them
// all as valid dependency roots for migration
pub const NUB_LOCK_NAMES: &[&str] = &[
    "nub.lock",
    "package-lock.json",
    "npm-shrinkwrap.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lock",
];

/// The package manager whose lockfile format and config files nub uses at
/// the root, from `packageManager`, then `devEngines.packageManager`, then
/// the lockfile. Nub owns repos without any.
/// https://nubjs.com/docs/install#lockfile-compatibility
pub fn detect_nub_identity(root: &VirtualPath) -> AnyResult<JavaScriptPackageManager> {
    let package = PackageJson::load(root.join("package.json"))?;

    for name in package.get_declared_package_managers() {
        if let Ok(package_manager) = JavaScriptPackageManager::from_str(name) {
            return Ok(package_manager);
        }
    }

    for (lock_name, package_manager) in [
        ("nub.lock", JavaScriptPackageManager::Nub),
        ("pnpm-lock.yaml", JavaScriptPackageManager::Pnpm),
        ("package-lock.json", JavaScriptPackageManager::Npm),
        ("npm-shrinkwrap.json", JavaScriptPackageManager::Npm),
        ("yarn.lock", JavaScriptPackageManager::Yarn),
        ("bun.lock", JavaScriptPackageManager::Bun),
    ] {
        if root.join(lock_name).exists() {
            return Ok(package_manager);
        }
    }

    Ok(JavaScriptPackageManager::Nub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::create_empty_sandbox;

    #[test]
    fn detects_the_identity() {
        for (files, expected) in [
            (vec![], JavaScriptPackageManager::Nub),
            (vec![("nub.lock", "")], JavaScriptPackageManager::Nub),
            (vec![("pnpm-lock.yaml", "")], JavaScriptPackageManager::Pnpm),
            (
                vec![("package-lock.json", "{}")],
                JavaScriptPackageManager::Npm,
            ),
            (vec![("yarn.lock", "")], JavaScriptPackageManager::Yarn),
            (vec![("bun.lock", "{}")], JavaScriptPackageManager::Bun),
            // Declarations win over lockfiles
            (
                vec![
                    ("pnpm-lock.yaml", ""),
                    ("package.json", r#"{ "packageManager": "nub@0.11.1" }"#),
                ],
                JavaScriptPackageManager::Nub,
            ),
            (
                vec![
                    ("nub.lock", ""),
                    (
                        "package.json",
                        r#"{ "devEngines": { "packageManager": { "name": "pnpm" } } }"#,
                    ),
                ],
                JavaScriptPackageManager::Pnpm,
            ),
            // Unknown names are skipped
            (
                vec![
                    ("yarn.lock", ""),
                    ("package.json", r#"{ "packageManager": "unknown@1.0.0" }"#),
                ],
                JavaScriptPackageManager::Yarn,
            ),
        ] {
            let sandbox = create_empty_sandbox();

            for (name, content) in &files {
                sandbox.create_file(name, content);
            }

            assert_eq!(
                detect_nub_identity(&VirtualPath::new(sandbox.path())).unwrap(),
                expected,
                "{files:?}"
            );
        }
    }
}
