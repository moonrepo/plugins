use moon_pdk_api::UnresolvedVersionSpec;

/// The tag of a toolchain's default Docker image for its configured version:
///
/// - A version or requirement is tagged without its operator and build metadata, like
///   `1.24` for `~1.24`, as images tag the newest release of each partial version.
/// - A lower bound, like `>=1.2.0`, and an upper bound that excludes its version, like `<2`,
///   are `latest`, as the version would be the oldest image, or one outside the range. An
///   upper bound like `<=1.4` keeps its version, the newest it allows.
/// - An alias, or `canary`, is the tag that `alias_tag` returns, otherwise `latest`.
/// - A range, a wildcard, and no version are `latest`.
pub fn get_docker_image_tag(
    version: Option<&UnresolvedVersionSpec>,
    alias_tag: impl Fn(&str) -> Option<String>,
) -> String {
    match version {
        Some(UnresolvedVersionSpec::Alias(alias)) => alias_tag(alias),
        Some(UnresolvedVersionSpec::Canary) => alias_tag("canary"),
        Some(version) => {
            let value = version.to_string();

            if value.starts_with('>') || value.starts_with('<') && !value.starts_with("<=") {
                None
            } else {
                Some(version.to_partial_string())
            }
        }
        None => None,
    }
    .unwrap_or_else(|| "latest".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(version: &str) -> String {
        get_docker_image_tag(
            Some(&UnresolvedVersionSpec::parse(version).unwrap()),
            |_| None,
        )
    }

    #[test]
    fn tags_versions_and_requirements_without_their_operator() {
        assert_eq!(tag("1.24.3"), "1.24.3");
        assert_eq!(tag("1.24"), "1.24");
        assert_eq!(tag("~1.24"), "1.24");
        assert_eq!(tag("^1.2"), "1.2");
        assert_eq!(tag("=1.2.3"), "1.2.3");
        assert_eq!(tag("1.2.3+build"), "1.2.3");
    }

    #[test]
    fn uses_latest_for_bounds_outside_their_version() {
        assert_eq!(tag(">=1.2.0"), "latest");
        assert_eq!(tag(">1.2"), "latest");
        assert_eq!(tag("<2"), "latest");
        assert_eq!(tag("<=1.4"), "1.4");
    }

    #[test]
    fn uses_latest_for_ranges_and_wildcards() {
        assert_eq!(tag(">=1.2 <2"), "latest");
        assert_eq!(tag("^1 || ^2"), "latest");
        assert_eq!(tag("*"), "latest");
        assert_eq!(get_docker_image_tag(None, |_| None), "latest");
    }

    #[test]
    fn maps_aliases_and_canary() {
        let alias_tag = |alias: &str| (alias == "lts").then(|| "lts".to_owned());

        assert_eq!(
            get_docker_image_tag(
                Some(&UnresolvedVersionSpec::parse("lts").unwrap()),
                alias_tag
            ),
            "lts"
        );
        assert_eq!(
            get_docker_image_tag(
                Some(&UnresolvedVersionSpec::parse("stable").unwrap()),
                alias_tag
            ),
            "latest"
        );
        assert_eq!(
            get_docker_image_tag(Some(&UnresolvedVersionSpec::Canary), |alias| Some(
                alias.into()
            )),
            "canary"
        );
        assert_eq!(
            get_docker_image_tag(Some(&UnresolvedVersionSpec::Canary), |_| None),
            "latest"
        );
    }
}
