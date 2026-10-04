use proto_pdk::*;
use regex::Regex;

pub fn versions_from_tags(tags: Vec<String>) -> AnyResult<LoadVersionsOutput> {
    // Kotlin also tags internal builds, IDE variants, and development snapshots.
    // Only expose releases whose version maps directly to a compiler ZIP name.
    let release = Regex::new(r"^v[0-9]+\.[0-9]+\.[0-9]+(?:-(?:Beta|RC|M)[0-9]*)?$")?;
    let mut versions = tags
        .into_iter()
        .filter(|tag| release.is_match(tag))
        .filter_map(|tag| tag.strip_prefix('v').map(str::to_owned))
        .collect::<Vec<_>>();

    versions.sort();
    versions.dedup();

    // The PDK otherwise falls back to a fictitious latest version of 0.0.0.
    if !versions.iter().any(|version| !version.contains('-')) {
        return Err(extism_pdk::Error::msg(
            "No stable Kotlin releases were found.",
        ));
    }

    Ok(LoadVersionsOutput::from(versions)?)
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
}
