use proto_pdk::{AnyResult, HostArch, HostEnvironment, HostOS, Version, VersionSpec, anyhow};
use serde::{Deserialize, Deserializer};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ZlsArtifact {
    pub shasum: String,
    pub tarball: String,
}

#[derive(Debug, Deserialize)]
pub struct ZlsRelease {
    #[serde(skip)]
    pub version: String,

    #[serde(flatten)]
    artifacts: HashMap<String, JsonValue>,
}

impl ZlsRelease {
    pub fn artifact(&self, env: &HostEnvironment) -> AnyResult<ZlsArtifact> {
        let targets = get_target_candidates(env)?;

        for target in &targets {
            if let Some(value) = self.artifacts.get(*target) {
                return Ok(serde_json::from_value(value.clone())?);
            }
        }

        Err(anyhow!(
            "No ZLS {} archive is available for target <id>{}</id>.",
            self.version,
            targets.join("</id> or <id>")
        ))
    }
}

#[derive(Debug)]
pub struct ZlsReleaseIndex(HashMap<String, ZlsRelease>);

impl<'de> Deserialize<'de> for ZlsReleaseIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut releases = HashMap::<String, ZlsRelease>::deserialize(deserializer)?;

        for (version, release) in &mut releases {
            release.version.clone_from(version);
        }

        Ok(Self(releases))
    }
}

impl ZlsReleaseIndex {
    pub fn stable_versions(&self) -> Vec<String> {
        let mut versions = self
            .0
            .keys()
            .filter(|version| {
                Version::parse(version).is_ok_and(|version| version.prerelease.is_none())
            })
            .cloned()
            .collect::<Vec<_>>();

        versions.sort();
        versions
    }

    pub fn find(&self, spec: &VersionSpec) -> AnyResult<&ZlsRelease> {
        if spec.is_latest() {
            return self.latest_stable().ok_or_else(|| {
                anyhow!("The ZLS release index does not contain a stable release.")
            });
        }

        let version = spec.to_string();

        self.0
            .get(&version)
            .ok_or_else(|| anyhow!("ZLS version <version>{version}</version> was not found."))
    }

    fn latest_stable(&self) -> Option<&ZlsRelease> {
        self.0
            .iter()
            .filter_map(|(version, release)| {
                Version::parse(version)
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
        _ => {
            return Err(anyhow!(
                "ZLS does not provide a pre-built archive for {} {}.",
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

    fn parse_index() -> ZlsReleaseIndex {
        serde_json::from_str(
            r#"{
                "0.16.0": {
                    "date": "2026-04-16",
                    "x86_64-linux": {
                        "tarball": "https://example.com/zls-0.16.0.tar.xz",
                        "shasum": "latest-hash"
                    }
                },
                "0.14.0": {
                    "date": "2025-01-08",
                    "armv7a-linux": {
                        "tarball": "https://example.com/zls-0.14.0.tar.xz",
                        "shasum": "stable-hash"
                    }
                },
                "nightly": {
                    "date": "2026-04-17"
                }
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn lists_only_stable_versions() {
        assert_eq!(parse_index().stable_versions(), ["0.14.0", "0.16.0"]);
    }

    #[test]
    fn finds_latest_and_exact_releases() {
        let index = parse_index();

        assert_eq!(
            index
                .find(&VersionSpec::parse("latest").unwrap())
                .unwrap()
                .version,
            "0.16.0"
        );
        assert_eq!(
            index
                .find(&VersionSpec::parse("0.14.0").unwrap())
                .unwrap()
                .version,
            "0.14.0"
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
