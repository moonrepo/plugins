use proto_pdk::{AnyResult, HostArch, HostEnvironment, HostOS, anyhow};
use serde::{Deserialize, Deserializer};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ReleaseArtifact {
    pub shasum: String,
    pub tarball: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZigProduct {
    Compiler,
    LanguageServer,
}

pub trait VersionedRelease {
    fn version_mut(&mut self) -> &mut String;
}

pub fn deserialize_release_map<'de, D, R>(deserializer: D) -> Result<HashMap<String, R>, D::Error>
where
    D: Deserializer<'de>,
    R: Deserialize<'de> + VersionedRelease,
{
    let mut releases = HashMap::<String, R>::deserialize(deserializer)?;

    for (version, release) in &mut releases {
        if release.version_mut().is_empty() {
            release.version_mut().clone_from(version);
        }
    }

    Ok(releases)
}

impl ZigProduct {
    fn name(self) -> &'static str {
        match self {
            Self::Compiler => "Zig",
            Self::LanguageServer => "ZLS",
        }
    }
}

pub fn select_release_artifact(
    artifacts: &HashMap<String, JsonValue>,
    version: &str,
    env: &HostEnvironment,
    product: ZigProduct,
) -> AnyResult<ReleaseArtifact> {
    let targets = target_candidates(env, product).ok_or_else(|| {
        anyhow!(
            "{} does not provide a pre-built archive for {} {}.",
            product.name(),
            env.os,
            env.arch
        )
    })?;

    for target in targets {
        if let Some(value) = artifacts.get(*target) {
            return Ok(serde_json::from_value(value.clone())?);
        }
    }

    Err(anyhow!(
        "No {} {} archive is available for target <id>{}</id>.",
        product.name(),
        version,
        targets.join("</id> or <id>")
    ))
}

fn target_candidates(
    env: &HostEnvironment,
    product: ZigProduct,
) -> Option<&'static [&'static str]> {
    let common: Option<&'static [&'static str]> = match (env.os, env.arch) {
        (HostOS::Linux, HostArch::X64) => Some(&["x86_64-linux"]),
        (HostOS::Linux, HostArch::X86) => Some(&["x86-linux"]),
        (HostOS::Linux, HostArch::Arm64) => Some(&["aarch64-linux"]),
        (HostOS::Linux, HostArch::Arm) => Some(&["arm-linux", "armv7a-linux"]),
        (HostOS::Linux, HostArch::LongArm64) => Some(&["loongarch64-linux"]),
        (HostOS::Linux, HostArch::Powerpc64) => Some(&["powerpc64le-linux"]),
        (HostOS::Linux, HostArch::Riscv64) => Some(&["riscv64-linux"]),
        (HostOS::Linux, HostArch::S390x) => Some(&["s390x-linux"]),
        (HostOS::MacOS, HostArch::X64) => Some(&["x86_64-macos"]),
        (HostOS::MacOS, HostArch::Arm64) => Some(&["aarch64-macos"]),
        (HostOS::Windows, HostArch::X64) => Some(&["x86_64-windows"]),
        (HostOS::Windows, HostArch::X86) => Some(&["x86-windows"]),
        (HostOS::Windows, HostArch::Arm64) => Some(&["aarch64-windows"]),
        _ => None,
    };

    common.or_else(|| -> Option<&'static [&'static str]> {
        if product != ZigProduct::Compiler {
            return None;
        }

        match (env.os, env.arch) {
            (HostOS::FreeBSD, HostArch::X64) => Some(&["x86_64-freebsd"]),
            (HostOS::FreeBSD, HostArch::Arm) => Some(&["arm-freebsd"]),
            (HostOS::FreeBSD, HostArch::Arm64) => Some(&["aarch64-freebsd"]),
            (HostOS::FreeBSD, HostArch::Powerpc64) => Some(&["powerpc64le-freebsd"]),
            (HostOS::FreeBSD, HostArch::Riscv64) => Some(&["riscv64-freebsd"]),
            (HostOS::NetBSD, HostArch::X64) => Some(&["x86_64-netbsd"]),
            (HostOS::NetBSD, HostArch::X86) => Some(&["x86-netbsd"]),
            (HostOS::NetBSD, HostArch::Arm) => Some(&["arm-netbsd"]),
            (HostOS::NetBSD, HostArch::Arm64) => Some(&["aarch64-netbsd"]),
            (HostOS::NetBSD, HostArch::Riscv64) => Some(&["riscv64-netbsd"]),
            (HostOS::OpenBSD, HostArch::X64) => Some(&["x86_64-openbsd"]),
            (HostOS::OpenBSD, HostArch::Arm) => Some(&["arm-openbsd"]),
            (HostOS::OpenBSD, HostArch::Arm64) => Some(&["aarch64-openbsd"]),
            (HostOS::OpenBSD, HostArch::Riscv64) => Some(&["riscv64-openbsd"]),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn supports_shared_and_legacy_targets() {
        let artifacts = HashMap::from([(
            "armv7a-linux".into(),
            json!({ "tarball": "https://example.com/zls.tar.xz", "shasum": "hash" }),
        )]);
        let env = HostEnvironment {
            os: HostOS::Linux,
            arch: HostArch::Arm,
            ..Default::default()
        };

        assert_eq!(
            select_release_artifact(&artifacts, "0.14.0", &env, ZigProduct::LanguageServer)
                .unwrap()
                .shasum,
            "hash"
        );
    }

    #[test]
    fn limits_bsd_targets_to_the_compiler() {
        let artifacts = HashMap::from([(
            "x86_64-freebsd".into(),
            json!({ "tarball": "https://example.com/zig.tar.xz", "shasum": "hash" }),
        )]);
        let env = HostEnvironment {
            os: HostOS::FreeBSD,
            arch: HostArch::X64,
            ..Default::default()
        };

        assert!(select_release_artifact(&artifacts, "0.14.0", &env, ZigProduct::Compiler).is_ok());
        assert!(
            select_release_artifact(&artifacts, "0.14.0", &env, ZigProduct::LanguageServer)
                .is_err()
        );
    }
}
