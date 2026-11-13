use proto_pdk::HostDistro;
use schematic::{ConfigEnum, derive_enum};

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
}
