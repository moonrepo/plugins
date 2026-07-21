use crate::{
    config::{ArchiveType, Distribution, JavaToolConfig, LibcType, PackageType, ReleaseType},
    java::JavaContext,
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
            ArchiveType::Tar
                | ArchiveType::TarGz
                | ArchiveType::TarXz
                | ArchiveType::TarZ
                | ArchiveType::Zip
        ) {
            return false;
        }

        true
    }
}

// The API only supports filtering by libc on some operating systems, and
// returns mixed results on others (e.g. glibc and musl for linux), so we
// must filter on our end to ensure a compatible package.
// https://github.com/foojayio/discoapi/issues/47
fn is_compatible_libc(package: &FoojayPackage, env: &HostEnvironment) -> bool {
    let base = if env.os.is_linux() {
        if env.libc == HostLibc::Musl {
            LibcType::Musl
        } else {
            LibcType::Glibc
        }
    } else if env.os.is_mac() {
        LibcType::Libc
    } else {
        LibcType::CStdLib
    };

    package.lib_c_type.as_ref().is_none_or(|libc| *libc == base)
}

pub fn find_package<'a>(
    packages: &'a [FoojayPackage],
    env: &HostEnvironment,
) -> Option<&'a FoojayPackage> {
    for archive in [
        ArchiveType::TarGz,
        ArchiveType::TarXz,
        ArchiveType::Tar,
        ArchiveType::Zip,
        // Always last since its non-standard
        ArchiveType::TarZ,
    ] {
        if let Some(package) = packages
            .iter()
            .find(|package| package.archive_type == archive && is_compatible_libc(package, env))
        {
            return Some(package);
        }
    }

    None
}

// https://github.com/foojayio/discoapi#endpoint-packages
pub fn fetch_packages(
    env: &HostEnvironment,
    config: &JavaToolConfig,
    java: &JavaContext,
) -> AnyResult<Vec<FoojayPackage>> {
    let mut url = format!(
        "{}/packages?latest=available&javafx_bundled=false&archive_type=tar&archive_type=tar.gz&archive_type=tar.xz&archive_type=tar.Z&archive_type=zip&distro={}&architecture={}&package_type={}&operating_system={}&release_status={}",
        config.api_url.trim_end_matches('/'),
        java.distribution.to_query_param(),
        java_arch(env)?,
        java.package,
        java_os(env)?,
        config.release_type.to_query_param(),
    );

    if !java.spec.is_latest() {
        url.push_str(&format!("&version={}", query_value(&java.short_version)));
    }

    let response: FoojayResponse<FoojayPackage> = fetch_json(&url)?;

    if response.result.is_empty() {
        return Err(PluginError::Message(format!(
            "No Java packages available (requested from <url>{url}</url>)."
        ))
        .into());
    }

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
    let url = format!("{}/ids/{id}", config.api_url.trim_end_matches('/'));
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
        HostOS::Linux => "linux",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_env(os: HostOS, libc: HostLibc) -> HostEnvironment {
        HostEnvironment {
            os,
            libc,
            ..HostEnvironment::default()
        }
    }

    fn create_package(archive_type: ArchiveType, lib_c_type: Option<LibcType>) -> FoojayPackage {
        FoojayPackage {
            archive_type,
            lib_c_type,
            operating_system: "linux".into(),
            ..FoojayPackage::default()
        }
    }

    mod supported_by_proto {
        use super::*;

        #[test]
        fn supports_standard_archives() {
            for archive_type in [
                ArchiveType::Tar,
                ArchiveType::TarGz,
                ArchiveType::TarXz,
                ArchiveType::TarZ,
                ArchiveType::Zip,
            ] {
                assert!(create_package(archive_type, None).is_supported_by_proto());
            }
        }

        #[test]
        fn doesnt_support_installers_or_system_packages() {
            for archive_type in [
                ArchiveType::Apk,
                ArchiveType::Cab,
                ArchiveType::Deb,
                ArchiveType::Dmg,
                ArchiveType::Exe,
                ArchiveType::Msi,
                ArchiveType::Pkg,
                ArchiveType::Rpm,
            ] {
                assert!(!create_package(archive_type, None).is_supported_by_proto());
            }
        }
    }

    mod compatible_libc {
        use super::*;

        #[test]
        fn linux_gnu_matches_glibc_only() {
            let env = create_env(HostOS::Linux, HostLibc::Gnu);

            assert!(is_compatible_libc(
                &create_package(ArchiveType::TarGz, Some(LibcType::Glibc)),
                &env
            ));
            assert!(!is_compatible_libc(
                &create_package(ArchiveType::TarGz, Some(LibcType::Musl)),
                &env
            ));
        }

        #[test]
        fn linux_musl_matches_musl_only() {
            let env = create_env(HostOS::Linux, HostLibc::Musl);

            assert!(is_compatible_libc(
                &create_package(ArchiveType::TarGz, Some(LibcType::Musl)),
                &env
            ));
            assert!(!is_compatible_libc(
                &create_package(ArchiveType::TarGz, Some(LibcType::Glibc)),
                &env
            ));
        }

        #[test]
        fn macos_matches_libc() {
            let env = create_env(HostOS::MacOS, HostLibc::Unknown);

            assert!(is_compatible_libc(
                &create_package(ArchiveType::TarGz, Some(LibcType::Libc)),
                &env
            ));
            assert!(!is_compatible_libc(
                &create_package(ArchiveType::TarGz, Some(LibcType::Glibc)),
                &env
            ));
        }

        #[test]
        fn windows_matches_c_std_lib() {
            let env = create_env(HostOS::Windows, HostLibc::Unknown);

            assert!(is_compatible_libc(
                &create_package(ArchiveType::Zip, Some(LibcType::CStdLib)),
                &env
            ));
        }

        #[test]
        fn missing_libc_always_matches() {
            for env in [
                create_env(HostOS::Linux, HostLibc::Gnu),
                create_env(HostOS::Linux, HostLibc::Musl),
                create_env(HostOS::MacOS, HostLibc::Unknown),
                create_env(HostOS::Windows, HostLibc::Unknown),
            ] {
                assert!(is_compatible_libc(
                    &create_package(ArchiveType::TarGz, None),
                    &env
                ));
            }
        }
    }

    mod find_package {
        use super::*;

        #[test]
        fn returns_none_when_empty() {
            let env = create_env(HostOS::Linux, HostLibc::Gnu);

            assert!(find_package(&[], &env).is_none());
        }

        #[test]
        fn prefers_tar_gz_over_zip() {
            let env = create_env(HostOS::Linux, HostLibc::Gnu);
            let packages = vec![
                create_package(ArchiveType::Zip, Some(LibcType::Glibc)),
                create_package(ArchiveType::TarGz, Some(LibcType::Glibc)),
            ];

            let package = find_package(&packages, &env).unwrap();

            assert_eq!(package.archive_type, ArchiveType::TarGz);
        }

        #[test]
        fn prefers_tar_z_last() {
            let env = create_env(HostOS::Linux, HostLibc::Gnu);
            let packages = vec![
                create_package(ArchiveType::TarZ, Some(LibcType::Glibc)),
                create_package(ArchiveType::Zip, Some(LibcType::Glibc)),
            ];

            let package = find_package(&packages, &env).unwrap();

            assert_eq!(package.archive_type, ArchiveType::Zip);
        }

        #[test]
        fn skips_incompatible_libc_within_archive_type() {
            let env = create_env(HostOS::Linux, HostLibc::Gnu);

            // The musl tar.gz comes first (like sap_machine responses),
            // and must be skipped in favor of the glibc variant
            let packages = vec![
                create_package(ArchiveType::TarGz, Some(LibcType::Musl)),
                create_package(ArchiveType::TarGz, Some(LibcType::Glibc)),
            ];

            let package = find_package(&packages, &env).unwrap();

            assert_eq!(package.lib_c_type, Some(LibcType::Glibc));
        }

        #[test]
        fn falls_through_archive_types_for_compatibility() {
            let env = create_env(HostOS::Linux, HostLibc::Gnu);

            // Only a musl tar.gz exists, so the glibc zip must win
            let packages = vec![
                create_package(ArchiveType::TarGz, Some(LibcType::Musl)),
                create_package(ArchiveType::Zip, Some(LibcType::Glibc)),
            ];

            let package = find_package(&packages, &env).unwrap();

            assert_eq!(package.archive_type, ArchiveType::Zip);
        }

        #[test]
        fn musl_host_picks_musl_variant() {
            let env = create_env(HostOS::Linux, HostLibc::Musl);
            let packages = vec![
                create_package(ArchiveType::TarGz, Some(LibcType::Glibc)),
                create_package(ArchiveType::TarGz, Some(LibcType::Musl)),
            ];

            let package = find_package(&packages, &env).unwrap();

            assert_eq!(package.lib_c_type, Some(LibcType::Musl));
        }
    }
}
