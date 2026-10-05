use crate::legacy::LEGACY_RELEASES;
use proto_pdk::*;

pub fn versions_from_tags(tags: Vec<String>) -> AnyResult<LoadVersionsOutput> {
    let mut versions = vec![];

    for tag in tags {
        let legacy = LEGACY_RELEASES
            .iter()
            .filter(|(_, release_tag, _)| *release_tag == tag)
            .map(|(version, _, _)| (*version).to_owned())
            .collect::<Vec<_>>();

        if !legacy.is_empty() {
            versions.extend(legacy);
            continue;
        }

        // These releases have no portable compiler asset.
        if matches!(
            tag.as_str(),
            "v1.4.21-2" | "v1.1.2-eap-73" | "v1.3.40-eap-21"
        ) {
            continue;
        }

        let Some(value) = tag.strip_prefix('v') else {
            continue;
        };
        let Ok(version) = Version::parse(value) else {
            continue;
        };

        // Accept numeric fixes and all preview identifiers, while excluding
        // scoped IDE variants and unpublished development snapshots.
        if version.scope.is_some()
            || version.prerelease.as_deref().is_some_and(|pre| {
                pre.split(['-', '.', '_']).any(|part| {
                    part.eq_ignore_ascii_case("dev") || part.eq_ignore_ascii_case("snapshot")
                })
            })
        {
            continue;
        }

        versions.push(version.to_string());
    }

    versions.sort();
    versions.dedup();

    // The PDK otherwise falls back to a fictitious latest version of 0.0.0.
    let output = LoadVersionsOutput::from(versions)?;

    if !output.versions.iter().any(|spec| {
        spec.as_version()
            .is_some_and(|version| version.prerelease.is_none() && version.build.is_none())
    }) {
        return Err(extism_pdk::Error::msg(
            "No stable Kotlin releases were found.",
        ));
    }

    Ok(output)
}

pub fn download_names(version: &Version) -> (String, String) {
    let version = version.to_string();

    if let Some((_, tag, archive_version)) = LEGACY_RELEASES
        .iter()
        .find(|(release_version, _, _)| *release_version == version)
    {
        (
            tag.to_string(),
            format!("kotlin-compiler-{archive_version}.zip"),
        )
    } else {
        (
            format!("v{version}"),
            format!("kotlin-compiler-{version}.zip"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_internal_tags_and_keeps_release_spelling() {
        let output = versions_from_tags(
            [
                "v1.9.25",
                "v2.4.10",
                "v2.4.20",
                "v2.4.20",
                "v2.5.0-Beta1",
                "v2.5.0-RC2",
                "v2.5.0-M1",
                "build-2.5.0-dev-123",
                "v2.5.0-dev-123",
                "v1.2.0_as31",
                "v2.6.0-SNAPSHOT",
                "v2.6",
                "2.6.0",
            ]
            .map(str::to_owned)
            .to_vec(),
        )
        .unwrap();

        assert_eq!(
            output
                .versions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [
                "1.9.25",
                "2.4.10",
                "2.4.20",
                "2.5.0-Beta1",
                "2.5.0-M1",
                "2.5.0-RC2"
            ]
        );
        assert_eq!(
            output.latest,
            Some(UnresolvedVersionSpec::parse("2.4.20").unwrap())
        );
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[test]
    fn rejects_empty_or_only_prerelease_catalogs() {
        assert!(versions_from_tags(vec![]).is_err());
        assert!(versions_from_tags(vec!["v2.5.0-Beta1".into()]).is_err());
    }

    #[test]
    fn includes_historical_releases_and_excludes_tags_without_compilers() {
        let output = versions_from_tags(
            [
                "build-0.6.31",
                "M11.1-bootstrap",
                "build-1.0.0",
                "1.0.1",
                "build-1.0.1",
                "1.0.1-2",
                "v1.0.5-2",
                "v1.1",
                "v1.1.4-3",
                "v1.2-M1",
                "v1.3-rc4",
                "v1.3.70-eap-274",
                "v1.4.0-rc",
                "v1.4.21-2",
                "v1.1.2-eap-73",
                "v1.3.40-eap-21",
                "build-1.0.0-dev-123",
            ]
            .map(str::to_owned)
            .to_vec(),
        )
        .unwrap();

        assert_eq!(
            output
                .versions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [
                "0.11.91+1",
                "0.6.31",
                "1.0.0",
                "1.0.1",
                "1.0.1-2",
                "1.0.5-2",
                "1.1.0",
                "1.1.4-3",
                "1.2.0-M1",
                "1.3.0-rc-190",
                "1.3.0-rc4",
                "1.3.70-eap-274",
                "1.4.0-rc",
            ]
        );
        assert_eq!(
            output.latest,
            Some(UnresolvedVersionSpec::parse("1.1.0").unwrap())
        );
    }

    #[test]
    fn preserves_published_tags_and_archive_names() {
        for (version, tag, archive_version) in [
            ("0.6.31", "build-0.6.31", "0.6.31"),
            ("0.7.395", "dot-operator", "0.7.395"),
            ("0.11.91+1", "M11.1-bootstrap", "0.11.91.1"),
            ("0.12.412", "build-0.12.412", "0.12.412.Idea141.1"),
            ("1.0.0", "build-1.0.0", "1.0.0"),
            ("1.0.1-2", "1.0.1-2", "1.0.1-2"),
            ("1.0.5-2", "v1.0.5-2", "1.0.5-2"),
            ("1.1.0", "v1.1", "1.1"),
            ("1.1.1-rc", "v1.1.1-rc", "1.1.1-eap-26"),
            ("1.1.1-eap-26", "v1.1.1-rc", "1.1.1-eap-26"),
            ("1.2.0-M1", "v1.2-M1", "1.2-M1"),
            ("1.3.0-rc4", "v1.3-rc4", "1.3.0-rc-190"),
            ("1.3.0-rc-190", "v1.3-rc4", "1.3.0-rc-190"),
            ("1.3.70-eap-274", "v1.3.70-eap-274", "1.3.70-eap-274"),
            ("1.4.0-rc", "v1.4.0-rc", "1.4.0-rc"),
        ] {
            assert_eq!(
                download_names(&Version::parse(version).unwrap()),
                (tag.into(), format!("kotlin-compiler-{archive_version}.zip")),
                "{version}"
            );
        }
    }
}
