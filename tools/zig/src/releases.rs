#![cfg_attr(not(feature = "wasm"), allow(dead_code))]

use proto_pdk::{AnyResult, HostArch, HostEnvironment, HostOS, Version, VersionSpec, anyhow};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ZigArtifact {
    pub shasum: String,
    pub tarball: String,
}

#[derive(Debug, Deserialize)]
pub struct ZigRelease {
    #[serde(default)]
    pub version: String,

    #[serde(flatten)]
    artifacts: HashMap<String, JsonValue>,
}

impl ZigRelease {
    pub fn artifact(&self, env: &HostEnvironment) -> AnyResult<ZigArtifact> {
        let targets = get_target_candidates(env)?;

        for target in &targets {
            if let Some(value) = self.artifacts.get(*target) {
                return Ok(serde_json::from_value(value.clone())?);
            }
        }

        Err(anyhow!(
            "No Zig {} archive is available for target <id>{}</id>.",
            self.version,
            targets.join("</id> or <id>")
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct ZigReleaseIndex(HashMap<String, ZigRelease>);

// impl<'de> Deserialize<'de> for ZigReleaseIndex {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let mut releases = HashMap::<String, ZigRelease>::deserialize(deserializer)?;

//         for (name, release) in &mut releases {
//             if release.version.is_empty() {
//                 release.version.clone_from(name);
//             }
//         }

//         Ok(Self(releases))
//     }
// }

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

fn get_target_candidates(env: &HostEnvironment) -> AnyResult<Vec<&'static str>> {
    let targets = match (env.os, env.arch) {
        (HostOS::Linux, HostArch::X64) => vec!["x86_64-linux"],
        (HostOS::Linux, HostArch::X86) => vec!["x86-linux"],
        (HostOS::Linux, HostArch::Arm64) => vec!["aarch64-linux"],
        (HostOS::Linux, HostArch::Arm) => vec!["arm-linux", "armv7a-linux"],
        (HostOS::Linux, HostArch::LongArm64) => vec!["loongarch64-linux"],
        (HostOS::Linux, HostArch::Powerpc64) => vec!["powerpc64le-linux"],
        (HostOS::Linux, HostArch::Riscv64) => vec!["riscv64-linux"],
        (HostOS::Linux, HostArch::S390x) => vec!["s390x-linux"],
        (HostOS::MacOS, HostArch::X64) => vec!["x86_64-macos"],
        (HostOS::MacOS, HostArch::Arm64) => vec!["aarch64-macos"],
        (HostOS::Windows, HostArch::X64) => vec!["x86_64-windows"],
        (HostOS::Windows, HostArch::X86) => vec!["x86-windows"],
        (HostOS::Windows, HostArch::Arm64) => vec!["aarch64-windows"],
        (HostOS::FreeBSD, HostArch::X64) => vec!["x86_64-freebsd"],
        (HostOS::FreeBSD, HostArch::Arm) => vec!["arm-freebsd"],
        (HostOS::FreeBSD, HostArch::Arm64) => vec!["aarch64-freebsd"],
        (HostOS::FreeBSD, HostArch::Powerpc64) => vec!["powerpc64le-freebsd"],
        (HostOS::FreeBSD, HostArch::Riscv64) => vec!["riscv64-freebsd"],
        (HostOS::NetBSD, HostArch::X64) => vec!["x86_64-netbsd"],
        (HostOS::NetBSD, HostArch::X86) => vec!["x86-netbsd"],
        (HostOS::NetBSD, HostArch::Arm) => vec!["arm-netbsd"],
        (HostOS::NetBSD, HostArch::Arm64) => vec!["aarch64-netbsd"],
        (HostOS::NetBSD, HostArch::Riscv64) => vec!["riscv64-netbsd"],
        (HostOS::OpenBSD, HostArch::X64) => vec!["x86_64-openbsd"],
        (HostOS::OpenBSD, HostArch::Arm) => vec!["arm-openbsd"],
        (HostOS::OpenBSD, HostArch::Arm64) => vec!["aarch64-openbsd"],
        (HostOS::OpenBSD, HostArch::Riscv64) => vec!["riscv64-openbsd"],
        _ => {
            return Err(anyhow!(
                "Zig does not provide a pre-built archive for {} {}.",
                env.os,
                env.arch
            ));
        }
    };

    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;

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
