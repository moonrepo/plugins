use proto_pdk::{
    AnyResult, HostArch, HostEnvironment, HostLibc, HostOS, UnresolvedVersionSpec, VersionSpec,
};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

pub mod v1;
pub mod v2;

pub const VERSION_REGEX: &str = r"^v?((?<major>[0-9]+)\.(?<minor>[0-9]+)\.(?<patch>[0-9]+)(?<pre>-[0-9a-zA-Z\.]+)?(?<build>\+[-0-9a-zA-Z\.]+)?)$";

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlatformMapper {
    pub arch: HashMap<HostArch, String>,
    pub archs: Vec<HostArch>,
    pub archive_prefix: Option<String>,
    pub checksum_name: Option<String>,
    pub download_name: Option<String>,
    pub exes_dirs: Option<Vec<PathBuf>>,
    pub exe_path: Option<PathBuf>,
    pub libc: HashMap<HostLibc, String>,
}

impl PlatformMapper {
    /// Returns the matching platform, and the OS it was matched with,
    /// which may be linux for BSD based OSes.
    pub fn find_match<'a>(
        platforms: &'a HashMap<HostOS, PlatformMapper>,
        env: &HostEnvironment,
    ) -> Option<(HostOS, &'a PlatformMapper)> {
        if let Some(platform) = platforms.get(&env.os) {
            return Some((env.os, platform));
        }

        // Fallback to linux for other OSes
        if env.os.is_bsd() {
            return platforms
                .get(&HostOS::Linux)
                .map(|platform| (HostOS::Linux, platform));
        }

        None
    }

    pub fn override_with(&mut self, other: &PlatformMapper) {
        self.arch.extend(other.arch.clone());
        self.libc.extend(other.libc.clone());

        if !other.archs.is_empty() {
            self.archs = other.archs.clone();
        }

        if let Some(value) = &other.exes_dirs {
            self.exes_dirs = Some(value.to_owned());
        }

        if let Some(value) = &other.download_name {
            self.download_name = Some(value.to_owned());
        }

        if let Some(value) = &other.archive_prefix {
            self.archive_prefix = Some(value.to_owned());
        }

        if let Some(value) = &other.checksum_name {
            self.checksum_name = Some(value.to_owned());
        }

        if let Some(value) = &other.exe_path {
            self.exe_path = Some(value.to_owned());
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum Schema {
    V1(v1::SchemaV1),
    V2(v2::SchemaV2),
}

impl Schema {
    pub fn get_name(&self) -> &str {
        match self {
            Self::V1(inner) => &inner.name,
            Self::V2(inner) => &inner.metadata.name,
        }
    }

    pub fn get_platform(
        &self,
        env: &HostEnvironment,
        spec: Option<&VersionSpec>,
    ) -> AnyResult<PlatformMapper> {
        let platform = match self {
            Self::V1(inner) => inner.get_platform(env)?,
            Self::V2(inner) => inner.get_platform(env, spec)?,
        };

        Ok(platform)
    }

    pub fn resolve_git_tag_pattern(&self) -> &str {
        match self {
            Self::V1(inner) => inner
                .resolve
                .git_tag_pattern
                .as_deref()
                .unwrap_or(&inner.resolve.version_pattern),
            Self::V2(inner) => inner
                .resolve
                .git_tag_pattern
                .as_deref()
                .unwrap_or(&inner.resolve.version_pattern),
        }
    }

    pub fn resolve_git_url(&self) -> Option<&str> {
        match self {
            Self::V1(inner) => inner.resolve.git_url.as_deref(),
            Self::V2(inner) => inner.resolve.git_url.as_deref(),
        }
    }

    pub fn resolve_manifest_url(&self) -> Option<&str> {
        match self {
            Self::V1(inner) => inner.resolve.manifest_url.as_deref(),
            Self::V2(inner) => inner.resolve.index_url.as_deref(),
        }
    }

    pub fn resolve_manifest_version_key(&self) -> &str {
        match self {
            Self::V1(inner) => &inner.resolve.manifest_version_key,
            Self::V2(inner) => &inner.resolve.index_version_key,
        }
    }

    pub fn resolve_manifest_version_pattern(&self) -> &str {
        match self {
            Self::V1(inner) => &inner.resolve.version_pattern,
            Self::V2(inner) => &inner.resolve.version_pattern,
        }
    }

    pub fn source_aliases(&self) -> HashMap<String, UnresolvedVersionSpec> {
        match self {
            Self::V1(inner) => inner.resolve.aliases.clone(),
            Self::V2(inner) => HashMap::from_iter(inner.source.aliases.clone()),
        }
    }

    pub fn source_latest(&self) -> Option<UnresolvedVersionSpec> {
        match self {
            Self::V1(_) => None,
            Self::V2(inner) => inner.source.latest.clone(),
        }
    }

    pub fn source_versions(&self) -> Vec<VersionSpec> {
        match self {
            Self::V1(inner) => inner.resolve.versions.clone(),
            Self::V2(inner) => inner.source.versions.clone(),
        }
    }
}

pub fn interpolate_tokens(
    value: &str,
    env: &HostEnvironment,
    spec: &VersionSpec,
    platform: &PlatformMapper,
) -> String {
    let arch = env.arch.to_rust_arch();
    let libc = env.libc.to_string();
    let os = env.os.to_rust_os();

    let mut value = value
        .replace("{version}", &spec.to_string())
        .replace("{arch}", platform.arch.get(&env.arch).unwrap_or(&arch))
        .replace("{libc}", platform.libc.get(&env.libc).unwrap_or(&libc))
        .replace("{os}", &os);

    if let Some(version) = spec.as_version() {
        let major = version.major.to_string();
        let minor = version.minor.to_string();
        let patch = version.patch.to_string();
        let year = format!("{:0>4}", version.major);
        let month = format!("{:0>2}", version.minor);
        let day = format!("{:0>2}", version.patch);
        let major_minor = format!("{}.{}", version.major, version.minor);
        let year_month = format!("{:0>4}-{:0>2}", version.major, version.minor);
        let pre = version
            .prerelease
            .as_ref()
            .map(|pre| pre.to_string())
            .unwrap_or_default();
        let build = version
            .build
            .as_ref()
            .map(|build| build.to_string())
            .unwrap_or_default();

        value = value
            .replace("{versionMajor}", &major)
            .replace("{versionMinor}", &minor)
            .replace("{versionPatch}", &patch)
            .replace("{versionMajorMinor}", &major_minor)
            .replace("{versionYear}", &year)
            .replace("{versionMonth}", &month)
            .replace("{versionDay}", &day)
            .replace("{versionYearMonth}", &year_month)
            .replace("{versionPrerelease}", &pre)
            .replace("{versionBuild}", &build);
    } else {
        value = value
            .replace("{versionMajor}", "")
            .replace("{versionMinor}", "")
            .replace("{versionPatch}", "")
            .replace("{versionMajorMinor}", "")
            .replace("{versionYear}", "")
            .replace("{versionMonth}", "")
            .replace("{versionDay}", "")
            .replace("{versionYearMonth}", "")
            .replace("{versionPrerelease}", "")
            .replace("{versionBuild}", "");
    }

    value
}
