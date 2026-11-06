use proto_pdk::{LoadVersionsOutput, UnresolvedVersionSpec, Version};

fn is_stable(version: &Version) -> bool {
    version.prerelease.is_none() && version.build.is_none()
}

/// Ensure the latest version has no pre-release or build metadata, by falling
/// back to the highest stable version. This is required when latest is sourced
/// externally (dist-tags, indexes, configs), or not set at all, as proto would
/// otherwise use the highest version. Also syncs the `latest` alias.
pub fn ensure_stable_latest(output: &mut LoadVersionsOutput) {
    let needs_fallback = match &output.latest {
        Some(UnresolvedVersionSpec::Version(version)) => !is_stable(version),
        Some(_) => false,
        None => true,
    };

    if needs_fallback
        && let Some(version) = output
            .versions
            .iter()
            .filter_map(|spec| spec.as_version())
            .filter(|version| is_stable(version))
            .max()
    {
        output.latest = Some(UnresolvedVersionSpec::Version(version.to_owned()));
    }

    if let Some(latest) = &output.latest {
        output.aliases.insert("latest".into(), latest.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto_pdk::VersionSpec;

    fn create_output(latest: Option<&str>, versions: &[&str]) -> LoadVersionsOutput {
        LoadVersionsOutput {
            latest: latest.map(|spec| UnresolvedVersionSpec::parse(spec).unwrap()),
            versions: versions
                .iter()
                .map(|spec| VersionSpec::parse(spec).unwrap())
                .collect(),
            ..Default::default()
        }
    }

    fn get_latest(output: &LoadVersionsOutput) -> Option<String> {
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());

        output.latest.as_ref().map(|spec| spec.to_string())
    }

    #[test]
    fn keeps_stable_latest() {
        let mut output = create_output(Some("1.0.0"), &["1.0.0", "2.0.0"]);
        ensure_stable_latest(&mut output);

        assert_eq!(get_latest(&output).unwrap(), "1.0.0");
    }

    #[test]
    fn replaces_prerelease_latest() {
        let mut output = create_output(Some("3.0.0-rc.1"), &["1.0.0", "2.0.0", "3.0.0-rc.1"]);
        ensure_stable_latest(&mut output);

        assert_eq!(get_latest(&output).unwrap(), "2.0.0");
    }

    #[test]
    fn replaces_build_metadata_latest() {
        let mut output = create_output(Some("2.0.0+build.5"), &["1.0.0", "2.0.0+build.5"]);
        ensure_stable_latest(&mut output);

        assert_eq!(get_latest(&output).unwrap(), "1.0.0");
    }

    #[test]
    fn sets_missing_latest() {
        let mut output = create_output(None, &["1.0.0", "2.0.0", "3.0.0-beta.1"]);
        ensure_stable_latest(&mut output);

        assert_eq!(get_latest(&output).unwrap(), "2.0.0");
    }

    #[test]
    fn keeps_unstable_latest_without_stable_versions() {
        let mut output = create_output(Some("1.0.0-rc.1"), &["1.0.0-rc.1"]);
        ensure_stable_latest(&mut output);

        assert_eq!(get_latest(&output).unwrap(), "1.0.0-rc.1");

        let mut output = create_output(None, &["1.0.0-rc.1"]);
        ensure_stable_latest(&mut output);

        assert_eq!(get_latest(&output), None);
    }

    #[test]
    fn keeps_non_version_latest() {
        let mut output = create_output(Some("^1"), &["1.0.0", "2.0.0"]);
        ensure_stable_latest(&mut output);

        assert_eq!(get_latest(&output).unwrap(), "^1");
    }
}
