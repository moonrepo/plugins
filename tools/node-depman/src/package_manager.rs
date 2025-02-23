#![allow(dead_code)]

use extism_pdk::Error;
use proto_pdk::{UnresolvedVersionSpec, get_plugin_id};
use std::fmt;

#[derive(PartialEq)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
}

impl PackageManager {
    pub fn detect() -> Result<PackageManager, Error> {
        let id = get_plugin_id()?;

        Ok(if id.to_lowercase().contains("yarn") {
            PackageManager::Yarn
        } else if id.to_lowercase().contains("pnpm") {
            PackageManager::Pnpm
        } else {
            PackageManager::Npm
        })
    }

    pub fn get_package_name(&self, version: impl AsRef<UnresolvedVersionSpec>) -> String {
        let version = version.as_ref();

        if matches!(self, PackageManager::Yarn) {
            if let UnresolvedVersionSpec::Semantic(inner) = &version {
                // Version 2.4.3 was published to the wrong package. It should
                // have been published to `@yarnpkg/cli-dist` but was published
                // to `yarn`. So... we need to manually fix it.
                // https://www.npmjs.com/package/yarn?activeTab=versions
                if inner.major == 2 && inner.minor == 4 && inner.patch == 3 {
                    return "yarn".into();
                }
            }

            if self.is_yarn_berry(version) {
                return "@yarnpkg/cli-dist".into();
            }
        }

        self.to_string()
    }

    pub fn is_yarn_classic(&self, version: impl AsRef<UnresolvedVersionSpec>) -> bool {
        matches!(self, PackageManager::Yarn)
            && match version.as_ref() {
                UnresolvedVersionSpec::Alias(alias) => alias == "legacy" || alias == "classic",
                UnresolvedVersionSpec::Semantic(ver) => ver.major == 1,
                UnresolvedVersionSpec::Req(req) => req.comparators.iter().any(|c| c.major == 1),
                _ => false,
            }
    }

    pub fn is_yarn_berry(&self, version: impl AsRef<UnresolvedVersionSpec>) -> bool {
        matches!(self, PackageManager::Yarn)
            && match version.as_ref() {
                UnresolvedVersionSpec::Alias(alias) => alias == "berry" || alias == "latest",
                UnresolvedVersionSpec::Semantic(ver) => ver.major > 1,
                UnresolvedVersionSpec::Req(req) => req.comparators.iter().any(|c| c.major > 1),
                _ => false,
            }
    }
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageManager::Npm => write!(f, "npm"),
            PackageManager::Pnpm => write!(f, "pnpm"),
            PackageManager::Yarn => write!(f, "yarn"),
        }
    }
}
