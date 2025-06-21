use proto_pdk_api::VersionSpec;

pub fn from_go_version(version: &str) -> String {
    // Zero releases don't end in ".0",
    // so we must fix manually...
    let suffix = match version.matches('.').count() {
        1 => ".0",
        0 => ".0.0",
        _ => "",
    };

    // go1.4rc1, go1.19.1beta, etc
    for id in ["alpha", "beta", "rc"] {
        let id_prefix = format!("{suffix}-{id}");

        if version.contains(id) && !version.contains(&id_prefix) {
            return version.replace(id, &id_prefix);
        }
    }

    format!("{version}{suffix}")
}

pub fn to_go_version(spec: &VersionSpec) -> String {
    match spec {
        VersionSpec::Canary => "canary".into(),
        VersionSpec::Alias(alias) => alias.to_string(),
        _ => {
            let version = spec.as_version().unwrap();

            // Versioning changed in >= 1.21.0
            // https://go.dev/doc/go1.21#introduction
            if version.major >= 1 && version.minor >= 21 {
                return version.to_string();
            }

            let mut next = version.to_string();

            // Remove all trailing ".0"
            #[allow(clippy::assigning_clones)]
            while let Some(prefix) = next.strip_suffix(".0") {
                next = prefix.to_owned();
            }

            // Remove leading ".0" from prereleases
            while next.contains(".0-") {
                next = next.replace(".0-", "-");
            }

            next.replace('-', "")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_from() {
        assert_eq!(from_go_version("1"), "1.0.0");
        assert_eq!(from_go_version("1.2"), "1.2.0");
        assert_eq!(from_go_version("1.2.3"), "1.2.3");

        assert_eq!(from_go_version("1alpha1"), "1.0.0-alpha1");
        assert_eq!(from_go_version("1.2beta2"), "1.2.0-beta2");
        assert_eq!(from_go_version("1.2.3rc3"), "1.2.3-rc3");

        // Shouldn't change
        assert_eq!(from_go_version("1.0.0"), "1.0.0");
        assert_eq!(from_go_version("1.0.0-alpha1"), "1.0.0-alpha1");
    }

    #[test]
    fn formats_to() {
        assert_eq!(to_go_version(&VersionSpec::parse("1.0.0").unwrap()), "1");
        assert_eq!(to_go_version(&VersionSpec::parse("1.2.0").unwrap()), "1.2");
        assert_eq!(
            to_go_version(&VersionSpec::parse("1.2.3").unwrap()),
            "1.2.3"
        );

        assert_eq!(
            to_go_version(&VersionSpec::parse("1.0.0-alpha1").unwrap()),
            "1alpha1"
        );
        assert_eq!(
            to_go_version(&VersionSpec::parse("1.2.0-beta2").unwrap()),
            "1.2beta2"
        );
        assert_eq!(
            to_go_version(&VersionSpec::parse("1.2.3-rc3").unwrap()),
            "1.2.3rc3"
        );

        // New versioning
        assert_eq!(
            to_go_version(&VersionSpec::parse("1.21.0").unwrap()),
            "1.21.0"
        );
        assert_eq!(
            to_go_version(&VersionSpec::parse("1.22.1").unwrap()),
            "1.22.1"
        );
        // assert_eq!(to_go_version("1.23.0-beta2"), "1.23.0beta2");
    }
}
