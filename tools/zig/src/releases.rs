#![cfg_attr(not(feature = "wasm"), allow(dead_code))]

use lang_zig_common::{
    ReleaseArtifact, VersionedRelease, ZigProduct, deserialize_release_map, select_release_artifact,
};
use proto_pdk_api::{AnyResult, HostEnvironment, Version, VersionSpec, anyhow};
use serde::{Deserialize, Deserializer};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ZigRelease {
    #[serde(default)]
    pub version: String,

    #[serde(flatten)]
    artifacts: HashMap<String, JsonValue>,
}

impl ZigRelease {
    pub fn artifact(&self, env: &HostEnvironment) -> AnyResult<ReleaseArtifact> {
        select_release_artifact(&self.artifacts, &self.version, env, ZigProduct::Compiler)
    }
}

impl VersionedRelease for ZigRelease {
    fn version_mut(&mut self) -> &mut String {
        &mut self.version
    }
}

#[derive(Debug)]
pub struct ZigReleaseIndex(HashMap<String, ZigRelease>);

impl<'de> Deserialize<'de> for ZigReleaseIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_release_map(deserializer).map(Self)
    }
}

impl ZigReleaseIndex {
    pub fn stable_versions(&self) -> Vec<String> {
        let mut versions = self
            .0
            .iter()
            .filter(|(name, _)| name.as_str() != "master")
            .map(|(_, release)| release.version.clone())
            .collect::<Vec<_>>();

        versions.sort();
        versions
    }

    pub fn master(&self) -> Option<&ZigRelease> {
        self.0.get("master")
    }

    pub fn find(&self, spec: &VersionSpec) -> AnyResult<&ZigRelease> {
        if spec.is_canary() || spec.is_alias("master") {
            return self
                .master()
                .ok_or_else(|| anyhow!("The Zig release index does not contain a master build."));
        }

        if spec.is_latest() {
            return self.latest_stable().ok_or_else(|| {
                anyhow!("The Zig release index does not contain a stable release.")
            });
        }

        let version = spec.to_string();

        self.0
            .get(&version)
            .or_else(|| self.0.values().find(|release| release.version == version))
            .ok_or_else(|| anyhow!("Zig version <version>{version}</version> was not found."))
    }

    fn latest_stable(&self) -> Option<&ZigRelease> {
        self.0
            .iter()
            .filter(|(name, _)| name.as_str() != "master")
            .filter_map(|(_, release)| {
                Version::parse(&release.version)
                    .ok()
                    .filter(|version| version.prerelease.is_none())
                    .map(|version| (version, release))
            })
            .max_by(|(a, _), (b, _)| a.cmp(b))
            .map(|(_, release)| release)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto_pdk_api::{HostArch, HostOS};

    fn parse_index() -> ZigReleaseIndex {
        serde_json::from_str(
            r#"{
                "master": {
                    "version": "0.15.0-dev.1+abc123",
                    "x86_64-linux": {
                        "tarball": "https://example.com/zig-master.tar.xz",
                        "shasum": "master-hash"
                    }
                },
                "0.14.0": {
                    "version": "0.14.0",
                    "armv7a-linux": {
                        "tarball": "https://example.com/zig-0.14.0.tar.xz",
                        "shasum": "stable-hash"
                    }
                },
                "0.13.0": {
                    "docs": "https://example.com/docs"
                }
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn lists_only_stable_versions() {
        assert_eq!(parse_index().stable_versions(), ["0.13.0", "0.14.0"]);
    }

    #[test]
    fn finds_master_and_stable_releases() {
        let index = parse_index();

        assert_eq!(
            index.find(&VersionSpec::Canary).unwrap().version,
            "0.15.0-dev.1+abc123"
        );
        assert_eq!(
            index
                .find(&VersionSpec::parse("latest").unwrap())
                .unwrap()
                .version,
            "0.14.0"
        );
        assert_eq!(
            index
                .find(&VersionSpec::parse("0.13.0").unwrap())
                .unwrap()
                .version,
            "0.13.0"
        );
    }

    #[test]
    fn supports_legacy_arm_target_names() {
        let index = parse_index();
        let release = index.find(&VersionSpec::parse("0.14.0").unwrap()).unwrap();
        let env = HostEnvironment {
            os: HostOS::Linux,
            arch: HostArch::Arm,
            ..Default::default()
        };

        assert_eq!(release.artifact(&env).unwrap().shasum, "stable-hash");
    }
}
