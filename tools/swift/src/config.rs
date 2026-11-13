use proto_pdk_api::{HostArch, HostDistro, Version};
use schematic::{ConfigEnum, derive_enum};

/// A Swift release as `(major, minor, patch)`.
type SwiftRelease = (u32, u32, u32);

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum LinuxPlatform {
        #[serde(rename = "amazon-linux-2", alias = "amazonlinux2")]
        AmazonLinux2,
        #[serde(rename = "amazon-linux-2023", alias = "amazonlinux2023")]
        AmazonLinux2023,
        #[serde(rename = "debian-12", alias = "debian12")]
        Debian12,
        #[serde(rename = "debian-13", alias = "debian13")]
        Debian13,
        #[serde(rename = "fedora-39", alias = "fedora39")]
        Fedora39,
        #[serde(rename = "fedora-41", alias = "fedora41")]
        Fedora41,
        #[serde(rename = "redhat-ubi-9", alias = "ubi9")]
        RedhatUbi9,
        #[serde(rename = "redhat-ubi-10", alias = "ubi10")]
        RedhatUbi10,
        #[serde(rename = "ubuntu-20.04", alias = "ubuntu2004")]
        Ubuntu2004,
        #[serde(rename = "ubuntu-22.04", alias = "ubuntu2204")]
        Ubuntu2204,
        #[default]
        #[serde(rename = "ubuntu-24.04", alias = "ubuntu2404")]
        Ubuntu2404,
        #[serde(rename = "ubuntu-26.04", alias = "ubuntu2604")]
        Ubuntu2604,
    }
);

impl LinuxPlatform {
    pub fn get_archive_suffix(&self) -> &'static str {
        match self {
            Self::AmazonLinux2 => "amazonlinux2",
            Self::AmazonLinux2023 => "amazonlinux2023",
            Self::Debian12 => "debian12",
            Self::Debian13 => "debian13",
            Self::Fedora39 => "fedora39",
            Self::Fedora41 => "fedora41",
            Self::RedhatUbi9 => "ubi9",
            Self::RedhatUbi10 => "ubi10",
            Self::Ubuntu2004 => "ubuntu20.04",
            Self::Ubuntu2204 => "ubuntu22.04",
            Self::Ubuntu2404 => "ubuntu24.04",
            Self::Ubuntu2604 => "ubuntu26.04",
        }
    }

    pub fn get_download_platform(&self) -> &'static str {
        match self {
            Self::AmazonLinux2 => "amazonlinux2",
            Self::AmazonLinux2023 => "amazonlinux2023",
            Self::Debian12 => "debian12",
            Self::Debian13 => "debian13",
            Self::Fedora39 => "fedora39",
            Self::Fedora41 => "fedora41",
            Self::RedhatUbi9 => "ubi9",
            Self::RedhatUbi10 => "ubi10",
            Self::Ubuntu2004 => "ubuntu2004",
            Self::Ubuntu2204 => "ubuntu2204",
            Self::Ubuntu2404 => "ubuntu2404",
            Self::Ubuntu2604 => "ubuntu2604",
        }
    }

    /// Match a Linux distribution to its build, like swiftly, where RHEL derivatives
    /// (Rocky, Alma, etc) use the UBI builds.
    pub fn from_distro(distro: &HostDistro) -> Option<Self> {
        let version = distro.version_id.as_deref().unwrap_or_default();

        Some(match (distro.id.as_str(), version) {
            ("ubuntu", "20.04") => Self::Ubuntu2004,
            ("ubuntu", "22.04") => Self::Ubuntu2204,
            ("ubuntu", "24.04") => Self::Ubuntu2404,
            ("ubuntu", "26.04") => Self::Ubuntu2604,
            ("debian", "12") => Self::Debian12,
            ("debian", "13") => Self::Debian13,
            ("fedora", "39") => Self::Fedora39,
            ("fedora", "41") => Self::Fedora41,
            ("amzn", "2") => Self::AmazonLinux2,
            ("amzn", "2023") => Self::AmazonLinux2023,
            _ if distro.id == "rhel" || distro.id_like.iter().any(|like| like == "rhel") => {
                match version.split('.').next() {
                    Some("9") => Self::RedhatUbi9,
                    Some("10") => Self::RedhatUbi10,
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    /// The first Swift release with a build for this platform and architecture,
    /// and the first release without one when the platform was dropped. Taken from
    /// <https://www.swift.org/api/v1/install/releases.json>.
    fn get_supported_releases(&self, arch: &HostArch) -> (SwiftRelease, Option<SwiftRelease>) {
        let arm64 = matches!(arch, HostArch::Arm64);

        match self {
            Self::AmazonLinux2 => (if arm64 { (5, 6, 0) } else { (5, 2, 4) }, Some((6, 4, 0))),
            Self::AmazonLinux2023 => ((6, 3, 1), None),
            Self::Debian12 => ((5, 10, 1), None),
            Self::Debian13 => ((6, 4, 0), None),
            Self::Fedora39 => ((5, 10, 1), Some((6, 4, 0))),
            Self::Fedora41 => ((6, 2, 4), None),
            Self::RedhatUbi9 => ((5, 8, 1), None),
            Self::RedhatUbi10 => ((6, 4, 0), None),
            Self::Ubuntu2004 => (if arm64 { (5, 6, 0) } else { (5, 2, 4) }, Some((6, 2, 0))),
            Self::Ubuntu2204 => ((5, 7, 0), None),
            Self::Ubuntu2404 => ((5, 10, 1), None),
            Self::Ubuntu2604 => ((6, 4, 0), None),
        }
    }

    /// Whether Swift published a build of the version for this platform.
    pub fn has_build_for(&self, version: &Version, arch: &HostArch) -> bool {
        let release = (version.major, version.minor, version.patch);
        let (first, dropped) = self.get_supported_releases(arch);

        release >= first && dropped.is_none_or(|dropped| release < dropped)
    }

    /// Older releases of the same distribution, newest first. Their builds
    /// also run on this release, as its system libraries are backwards compatible.
    fn get_older_releases(&self) -> &'static [Self] {
        match self {
            Self::AmazonLinux2023 => &[Self::AmazonLinux2],
            Self::Debian13 => &[Self::Debian12],
            Self::Fedora41 => &[Self::Fedora39],
            Self::RedhatUbi10 => &[Self::RedhatUbi9],
            Self::Ubuntu2204 => &[Self::Ubuntu2004],
            Self::Ubuntu2404 => &[Self::Ubuntu2204, Self::Ubuntu2004],
            Self::Ubuntu2604 => &[Self::Ubuntu2404, Self::Ubuntu2204, Self::Ubuntu2004],
            Self::AmazonLinux2
            | Self::Debian12
            | Self::Fedora39
            | Self::RedhatUbi9
            | Self::Ubuntu2004 => &[],
        }
    }

    /// Select the build of a Swift version for this platform. When the version
    /// doesn't have one, like Swift 6.1 for Ubuntu 26.04, the build for the
    /// newest older release of the same distribution is used instead, as Swift
    /// only builds each version for the distribution releases current at the time.
    pub fn select_build_for(&self, version: &Version, arch: &HostArch) -> Option<Self> {
        std::iter::once(self)
            .chain(self.get_older_releases())
            .find(|platform| platform.has_build_for(version, arch))
            .cloned()
    }
}

#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct SwiftToolConfig {
    pub dist_url: String,
    /// Detected from `/etc/os-release` when not configured.
    pub linux_platform: Option<LinuxPlatform>,
}

impl Default for SwiftToolConfig {
    fn default() -> Self {
        Self {
            dist_url: "https://download.swift.org/{release}/{platform}/{folder}/{file}".into(),
            linux_platform: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_platform_from_os_release() {
        for (content, expected) in [
            ("ID=ubuntu\nVERSION_ID=\"24.04\"", LinuxPlatform::Ubuntu2404),
            ("ID=ubuntu\nVERSION_ID=\"22.04\"", LinuxPlatform::Ubuntu2204),
            ("ID=debian\nVERSION_ID=\"13\"", LinuxPlatform::Debian13),
            ("ID=fedora\nVERSION_ID=41", LinuxPlatform::Fedora41),
            (
                "ID=\"amzn\"\nID_LIKE=\"centos rhel fedora\"\nVERSION_ID=\"2\"",
                LinuxPlatform::AmazonLinux2,
            ),
            (
                "ID=\"amzn\"\nID_LIKE=\"fedora\"\nVERSION_ID=\"2023\"",
                LinuxPlatform::AmazonLinux2023,
            ),
            ("ID=\"rhel\"\nVERSION_ID=\"9.4\"", LinuxPlatform::RedhatUbi9),
            (
                "ID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\nVERSION_ID=\"10.0\"",
                LinuxPlatform::RedhatUbi10,
            ),
        ] {
            assert_eq!(
                HostDistro::parse_os_release(content)
                    .and_then(|distro| LinuxPlatform::from_distro(&distro)),
                Some(expected),
                "for {content:?}"
            );
        }
    }

    #[test]
    fn doesnt_detect_unsupported_platforms() {
        for content in [
            "",
            "ID=ubuntu\nVERSION_ID=\"18.04\"",
            "ID=fedora\nVERSION_ID=40",
            "ID=linuxmint\nID_LIKE=\"ubuntu debian\"\nVERSION_ID=\"22\"",
            "ID=alpine\nVERSION_ID=3.20.0",
            "ID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\nVERSION_ID=\"8.10\"",
        ] {
            assert_eq!(
                HostDistro::parse_os_release(content)
                    .and_then(|distro| LinuxPlatform::from_distro(&distro)),
                None,
                "for {content:?}"
            );
        }
    }

    fn select(platform: LinuxPlatform, version: &str, arch: HostArch) -> Option<LinuxPlatform> {
        platform.select_build_for(&Version::parse(version).unwrap(), &arch)
    }

    #[test]
    fn selects_platform_when_version_has_a_build() {
        for (platform, version) in [
            (LinuxPlatform::Ubuntu2604, "6.4.0"),
            (LinuxPlatform::Ubuntu2404, "5.10.1"),
            (LinuxPlatform::Ubuntu2004, "6.1.3"),
            (LinuxPlatform::Debian13, "6.4.2"),
            (LinuxPlatform::Fedora41, "6.2.4"),
            (LinuxPlatform::AmazonLinux2023, "6.3.1"),
            (LinuxPlatform::RedhatUbi10, "6.4.0"),
            // Versions newer than the data keep current platforms
            (LinuxPlatform::Ubuntu2404, "7.0.0"),
        ] {
            assert_eq!(
                select(platform.clone(), version, HostArch::X64),
                Some(platform.clone()),
                "for {platform} {version}"
            );
        }
    }

    #[test]
    fn selects_older_release_when_version_has_no_build() {
        for (platform, version, expected) in [
            (
                LinuxPlatform::Ubuntu2604,
                "6.1.3",
                LinuxPlatform::Ubuntu2404,
            ),
            (
                LinuxPlatform::Ubuntu2604,
                "6.3.3",
                LinuxPlatform::Ubuntu2404,
            ),
            (
                LinuxPlatform::Ubuntu2404,
                "5.9.2",
                LinuxPlatform::Ubuntu2204,
            ),
            (
                LinuxPlatform::Ubuntu2404,
                "5.6.3",
                LinuxPlatform::Ubuntu2004,
            ),
            (LinuxPlatform::Debian13, "6.3.3", LinuxPlatform::Debian12),
            (LinuxPlatform::Fedora41, "6.2.3", LinuxPlatform::Fedora39),
            (
                LinuxPlatform::AmazonLinux2023,
                "6.3.0",
                LinuxPlatform::AmazonLinux2,
            ),
            (
                LinuxPlatform::RedhatUbi10,
                "6.1.3",
                LinuxPlatform::RedhatUbi9,
            ),
        ] {
            assert_eq!(
                select(platform.clone(), version, HostArch::X64),
                Some(expected),
                "for {platform} {version}"
            );
        }
    }

    #[test]
    fn checks_arm64_separately() {
        assert_eq!(
            select(LinuxPlatform::Ubuntu2004, "5.5.0", HostArch::X64),
            Some(LinuxPlatform::Ubuntu2004)
        );
        assert_eq!(
            select(LinuxPlatform::Ubuntu2004, "5.5.0", HostArch::Arm64),
            None
        );
        assert_eq!(
            select(LinuxPlatform::Ubuntu2004, "5.6.0", HostArch::Arm64),
            Some(LinuxPlatform::Ubuntu2004)
        );
    }

    #[test]
    fn doesnt_select_without_a_build_for_any_release() {
        for (platform, version) in [
            // Dropped releases
            (LinuxPlatform::Ubuntu2004, "6.2.0"),
            (LinuxPlatform::Fedora39, "6.4.0"),
            (LinuxPlatform::AmazonLinux2, "6.4.2"),
            // Before the oldest release had a build
            (LinuxPlatform::Debian13, "5.9.2"),
            (LinuxPlatform::RedhatUbi10, "5.7.3"),
            (LinuxPlatform::Ubuntu2604, "5.1.0"),
        ] {
            assert_eq!(
                select(platform.clone(), version, HostArch::X64),
                None,
                "for {platform} {version}"
            );
        }
    }
}
