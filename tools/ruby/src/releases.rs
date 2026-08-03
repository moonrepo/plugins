use proto_pdk::*;
use std::collections::BTreeMap;

pub type PrebuiltReleases = BTreeMap<String, BTreeMap<String, String>>;

#[derive(Debug, PartialEq)]
pub struct PrebuiltAsset {
    pub filename: String,
    pub url: String,
}

pub fn load_prebuilt_asset(
    env: &HostEnvironment,
    version: &VersionSpec,
) -> AnyResult<Option<PrebuiltAsset>> {
    let Some(platform) = get_prebuilt_platform(env) else {
        return Ok(None);
    };

    let releases: PrebuiltReleases = fetch_json(
        "https://raw.githubusercontent.com/moonrepo/plugins/master/tools/ruby/releases.json",
    )?;

    Ok(select_prebuilt_asset(&releases, platform, version))
}

pub fn select_prebuilt_asset(
    releases: &PrebuiltReleases,
    platform: &str,
    version: &VersionSpec,
) -> Option<PrebuiltAsset> {
    let filename = releases.get(&version.to_string())?.get(platform)?;

    Some(PrebuiltAsset {
        filename: filename.to_owned(),
        url: format!("https://github.com/jdx/ruby/releases/download/{version}/{filename}"),
    })
}

pub fn create_download_output(
    asset: PrebuiltAsset,
    version: &VersionSpec,
) -> DownloadPrebuiltOutput {
    DownloadPrebuiltOutput {
        archive_prefix: Some(format!("ruby-{version}")),
        download_name: Some(asset.filename),
        download_url: asset.url,
        ..Default::default()
    }
}

pub fn get_prebuilt_platform(env: &HostEnvironment) -> Option<&'static str> {
    match (env.os, env.arch) {
        (HostOS::Linux, HostArch::X64) => Some("x86_64_linux"),
        (HostOS::Linux, HostArch::Arm64) => Some("arm64_linux"),
        (HostOS::MacOS, HostArch::Arm64) => Some("macos"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_jdx_supported_platforms() {
        for (os, arch, expected) in [
            (HostOS::Linux, HostArch::X64, Some("x86_64_linux")),
            (HostOS::Linux, HostArch::Arm64, Some("arm64_linux")),
            (HostOS::MacOS, HostArch::Arm64, Some("macos")),
            (HostOS::MacOS, HostArch::X64, None),
            (HostOS::Windows, HostArch::X64, None),
        ] {
            assert_eq!(
                get_prebuilt_platform(&HostEnvironment {
                    os,
                    arch,
                    ..Default::default()
                }),
                expected
            );
        }
    }

    #[test]
    fn selects_matching_release_asset() {
        let asset = select_prebuilt_asset(
            &BTreeMap::from_iter([(
                "3.4.9".into(),
                BTreeMap::from_iter([(
                    "arm64_linux".into(),
                    "ruby-3.4.9.arm64_linux.tar.gz".into(),
                )]),
            )]),
            "arm64_linux",
            &VersionSpec::parse("3.4.9").unwrap(),
        );

        assert_eq!(
            asset,
            Some(PrebuiltAsset {
                filename: "ruby-3.4.9.arm64_linux.tar.gz".into(),
                url: "https://github.com/jdx/ruby/releases/download/3.4.9/ruby-3.4.9.arm64_linux.tar.gz".into(),
            })
        );
    }

    #[test]
    fn skips_release_without_matching_asset() {
        let asset = select_prebuilt_asset(
            &BTreeMap::from_iter([("3.4.9".into(), BTreeMap::new())]),
            "macos",
            &VersionSpec::parse("3.4.9").unwrap(),
        );

        assert_eq!(asset, None);
    }

    #[test]
    fn skips_missing_release() {
        let asset = select_prebuilt_asset(
            &BTreeMap::new(),
            "macos",
            &VersionSpec::parse("3.1.0").unwrap(),
        );

        assert_eq!(asset, None);
    }

    #[test]
    fn creates_download_output() {
        assert_eq!(
            create_download_output(
                PrebuiltAsset {
                    filename: "ruby-3.4.9.macos.tar.gz".into(),
                    url: "https://example.com/ruby-3.4.9.macos.tar.gz".into(),
                },
                &VersionSpec::parse("3.4.9").unwrap(),
            ),
            DownloadPrebuiltOutput {
                archive_prefix: Some("ruby-3.4.9".into()),
                download_name: Some("ruby-3.4.9.macos.tar.gz".into()),
                download_url: "https://example.com/ruby-3.4.9.macos.tar.gz".into(),
                ..Default::default()
            }
        );
    }
}
