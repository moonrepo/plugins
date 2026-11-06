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
}

#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct SwiftToolConfig {
    pub dist_url: String,
    pub linux_platform: LinuxPlatform,
}

impl Default for SwiftToolConfig {
    fn default() -> Self {
        Self {
            dist_url: "https://download.swift.org/{release}/{platform}/{folder}/{file}".into(),
            linux_platform: LinuxPlatform::default(),
        }
    }
}
