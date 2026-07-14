use crate::config::{
    ArchiveType, Distribution, JavaToolConfig, LibcType, PackageType, ReleaseType,
};
use proto_pdk::{AnyResult, HostArch, HostEnvironment, HostLibc, HostOS, PluginError, fetch_json};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FoojayResponse<T> {
    result: Vec<T>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FoojayPackage {
    pub architecture: String,
    pub archive_type: ArchiveType,
    pub distribution: Distribution,
    pub distribution_version: String,
    pub id: String,
    pub java_version: String,
    pub lib_c_type: Option<LibcType>,
    pub operating_system: String,
    pub package_type: PackageType,
    pub release_status: ReleaseType,
}

impl FoojayPackage {
    pub fn is_supported_by_proto(&self) -> bool {
        // foojay supports dmg/pkg as well, but they are not as
        // compatible with proto as standard archives are
        if !matches!(
            self.archive_type,
            ArchiveType::Tar | ArchiveType::TarGz | ArchiveType::TarXz | ArchiveType::Zip
        ) {
            return false;
        }

        if self.operating_system == "linux"
            && self
                .lib_c_type
                .as_ref()
                .is_some_and(|libc| *libc == LibcType::CStdLib)
        {
            return false;
        }

        true
    }
}

// https://github.com/foojayio/discoapi#endpoint-packages
pub fn fetch_packages(
    env: &HostEnvironment,
    config: &JavaToolConfig,
    version: Option<&str>,
) -> AnyResult<Vec<FoojayPackage>> {
    let mut url = format!(
        "{}/packages?latest=available&directly_downloadable=true&javafx_bundled=false&distro={}&architecture={}&package_type={}&operating_system={}&release_status={}",
        config.api_url.trim_end_matches('/'),
        config.distribution.to_query_param(),
        java_arch(env)?,
        config.package_type.to_string(),
        java_os(env)?,
        config.release_type.to_query_param(),
    );

    if let Some(version) = version {
        url.push_str(&format!("&version={}", query_value(version)));
    }

    let response: FoojayResponse<FoojayPackage> = fetch_json(url)?;

    Ok(response
        .result
        .into_iter()
        .filter(|package| package.is_supported_by_proto())
        .collect())
}

// Note: The API returns these fields as empty strings
// instead of omitting them entirely!
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FoojayPackageInfo {
    pub checksum: String,
    pub checksum_type: String,
    pub checksum_uri: String,
    pub direct_download_uri: String,
    pub filename: String,
    pub signature_uri: String,
}

impl FoojayPackageInfo {
    pub fn is_checksum_supported_by_proto(&self) -> bool {
        !self.checksum.is_empty()
            && (self.checksum_type == "sha256" || self.checksum_type == "sha512")
    }
}

// https://github.com/foojayio/discoapi#endpoint-packages
pub fn fetch_package_info(config: &JavaToolConfig, id: &str) -> AnyResult<FoojayPackageInfo> {
    let url = format!("{}/ids/{id}", config.api_url.trim_end_matches('/'),);
    let mut response: FoojayResponse<FoojayPackageInfo> = fetch_json(&url)?;

    if response.result.len() != 1 {
        return Err(PluginError::Message(format!(
            "No Java package information found for ID <id>{id}</id> (requested from <url>{url}</url>)."
        ))
        .into());
    }

    Ok(response.result.remove(0))
}

fn query_value(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .replace('%', "%25")
        .replace('+', "%2B")
        .replace(' ', "%20")
}

fn java_os(env: &HostEnvironment) -> AnyResult<&'static str> {
    Ok(match env.os {
        HostOS::Linux => {
            if env.libc == HostLibc::Musl {
                "linux_musl"
            } else {
                "linux"
            }
        }
        HostOS::MacOS => "macos",
        HostOS::Windows => "windows",
        _ => {
            return Err(PluginError::UnsupportedOS {
                tool: "Java".into(),
                os: env.os.to_string(),
            }
            .into());
        }
    })
}

fn java_arch(env: &HostEnvironment) -> AnyResult<&'static str> {
    Ok(match env.arch {
        HostArch::X64 => "x64",
        HostArch::X86 => "x86",
        HostArch::Arm => "arm",
        HostArch::Arm64 => "aarch64",
        HostArch::Mips => "mips",
        HostArch::Powerpc => "ppc",
        HostArch::Powerpc64 => "ppc64",
        HostArch::Riscv64 => "riscv64",
        HostArch::S390x => "s390x",
        HostArch::Sparc64 => "sparcv9",
        _ => {
            return Err(PluginError::UnsupportedArch {
                tool: "Java".into(),
                arch: env.arch.to_string(),
            }
            .into());
        }
    })
}
