use proto_pdk_api::VersionSpec;

pub fn from_java_version(version: &str) -> String {
    let (prefix, build) = if version.contains('+') {
        version.split_once('+').unwrap()
    } else {
        (version, "")
    };

    // Versions don't end in trailing ".0",
    // so we must fix manually...
    let suffix = match prefix.matches('.').count() {
        1 => ".0",
        0 => ".0.0",
        _ => "",
    };

    let mut result = format!("{prefix}{suffix}");

    if !build.is_empty() {
        result.push('+');
        result.push_str(build);
    }

    result
}

pub fn to_java_version(spec: &VersionSpec) -> String {
    match spec {
        VersionSpec::Canary => "canary".into(),
        VersionSpec::Alias(alias) => alias.to_string(),
        _ => {
            let version = spec.as_version().unwrap();
            let mut next = version.major.to_string();

            if version.minor > 0 {
                next.push('.');
                next.push_str(&version.minor.to_string());

                if version.patch > 0 {
                    next.push('.');
                    next.push_str(&version.patch.to_string());
                }
            }

            if !version.pre.is_empty() {
                next = format!("{next}-{}", version.pre);
            }

            if !version.build.is_empty() {
                next = format!("{next}+{}", version.build);
            }

            next
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_from() {
        assert_eq!(from_java_version("1"), "1.0.0");
        assert_eq!(from_java_version("1.2"), "1.2.0");
        assert_eq!(from_java_version("1.2.3"), "1.2.3");

        assert_eq!(from_java_version("1+1"), "1.0.0+1");
        assert_eq!(from_java_version("1.2+2"), "1.2.0+2");
        assert_eq!(from_java_version("1.2.3+3"), "1.2.3+3");

        // Shouldn't change
        assert_eq!(from_java_version("1.0.0"), "1.0.0");
        assert_eq!(from_java_version("1.0.0-alpha1"), "1.0.0-alpha1");
    }

    #[test]
    fn formats_to() {
        assert_eq!(to_java_version(&VersionSpec::parse("1.0.0").unwrap()), "1");
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.0").unwrap()),
            "1.2"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.3").unwrap()),
            "1.2.3"
        );

        assert_eq!(
            to_java_version(&VersionSpec::parse("1.0.0+1").unwrap()),
            "1+1"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.0+2").unwrap()),
            "1.2+2"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.3+3").unwrap()),
            "1.2.3+3"
        );
    }
}
