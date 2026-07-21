use proto_pdk_api::VersionSpec;

pub fn from_java_version(version: &str) -> String {
    let mut value = version;
    let mut pre = None;
    let mut build = None;

    if let Some(i) = value.rfind('+') {
        build = Some(&value[i + 1..]);
        value = &value[0..i];
    }

    if let Some(i) = value.find('-') {
        pre = Some(&value[i + 1..]);
        value = &value[0..i];
    }

    let mut parts = value.split('.');
    let mut out = String::new();

    // major
    out.push_str(parts.next().expect("Expected a major version"));

    // minor
    out.push('.');
    out.push_str(parts.next().unwrap_or("0"));

    // patch
    out.push('.');
    out.push_str(parts.next().unwrap_or("0"));

    // A 4th "vendor" component (18.0.2.1) can't be represented in semver,
    // and the scope position is reserved for the distribution, so encode
    // it into the pre-release as `vN`. `to_java_version` decodes it back.
    let vendor = parts.next();

    if pre.is_some() || vendor.is_some() {
        out.push('-');

        if let Some(pre) = pre {
            out.push_str(pre);

            if let Some(vendor) = vendor {
                out.push_str(".v");
                out.push_str(vendor);
            }
        } else if let Some(vendor) = vendor {
            out.push('v');
            out.push_str(vendor);
        }
    }

    if let Some(build) = build {
        out.push('+');
        out.push_str(build);
    }

    out
}

pub fn to_java_version(spec: &VersionSpec) -> String {
    match spec {
        VersionSpec::Canary => "canary".into(),
        VersionSpec::Alias(alias) => alias.to_string(),
        _ => {
            let version = spec.as_version().unwrap();

            // Decode a `vN`-encoded 4th "vendor" component from the
            // pre-release (see `from_java_version`)
            fn is_digits(value: &str) -> bool {
                !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
            }

            let (pre, vendor) = match &version.prerelease {
                Some(pre) => match pre.split_once(".v") {
                    Some((head, tail)) if is_digits(tail) => {
                        (Some(head.to_owned()), Some(tail.to_owned()))
                    }
                    _ => match pre.strip_prefix('v') {
                        Some(tail) if is_digits(tail) => (None, Some(tail.to_owned())),
                        _ => (Some(pre.to_string()), None),
                    },
                },
                None => (None, None),
            };

            let mut out = version.major.to_string();

            if version.minor > 0 || version.patch > 0 || vendor.is_some() {
                out.push('.');
                out.push_str(&version.minor.to_string());

                if version.patch > 0 || vendor.is_some() {
                    out.push('.');
                    out.push_str(&version.patch.to_string());
                }
            }

            if let Some(vendor) = vendor {
                out.push('.');
                out.push_str(&vendor);
            }

            if let Some(pre) = pre {
                out = format!("{out}-{pre}");
            }

            if let Some(build) = &version.build {
                out = format!("{out}+{build}");
            }

            out
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
        assert_eq!(from_java_version("1.2.3.4"), "1.2.3-v4");

        assert_eq!(from_java_version("1-ea"), "1.0.0-ea");
        assert_eq!(from_java_version("1.2-ea"), "1.2.0-ea");
        assert_eq!(from_java_version("1.2.3-ea"), "1.2.3-ea");
        assert_eq!(from_java_version("1.2.3.4-ea"), "1.2.3-ea.v4");

        assert_eq!(from_java_version("1+1"), "1.0.0+1");
        assert_eq!(from_java_version("1.2+2"), "1.2.0+2");
        assert_eq!(from_java_version("1.2.3+3"), "1.2.3+3");
        assert_eq!(from_java_version("1.2.3.4+4"), "1.2.3-v4+4");

        assert_eq!(from_java_version("1-ea+1"), "1.0.0-ea+1");
        assert_eq!(from_java_version("1.2-ea+2"), "1.2.0-ea+2");
        assert_eq!(from_java_version("1.2.3-ea+3"), "1.2.3-ea+3");
        assert_eq!(from_java_version("1.2.3.4-ea+4"), "1.2.3-ea.v4+4");

        // Shouldn't change
        assert_eq!(from_java_version("1.0.0"), "1.0.0");
        assert_eq!(from_java_version("1.0.0-alpha1"), "1.0.0-alpha1");

        // Real-world versions
        assert_eq!(from_java_version("21.0.11+10"), "21.0.11+10");
        assert_eq!(from_java_version("8.0.492+9"), "8.0.492+9");
        assert_eq!(from_java_version("18.0.2.1+1"), "18.0.2-v1+1");
    }

    #[test]
    fn formats_to() {
        assert_eq!(to_java_version(&VersionSpec::parse("1.0.0").unwrap()), "1");
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.0.1").unwrap()),
            "1.0.1"
        );

        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.0").unwrap()),
            "1.2"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.3").unwrap()),
            "1.2.3"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.3-v4").unwrap()),
            "1.2.3.4"
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
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.3-v4+4").unwrap()),
            "1.2.3.4+4"
        );

        assert_eq!(
            to_java_version(&VersionSpec::parse("1.0.0-ea+1").unwrap()),
            "1-ea+1"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.0-ea+2").unwrap()),
            "1.2-ea+2"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.3-ea+3").unwrap()),
            "1.2.3-ea+3"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse("1.2.3-ea.v4+4").unwrap()),
            "1.2.3.4-ea+4"
        );

        // Real-world round trips
        assert_eq!(
            to_java_version(&VersionSpec::parse(from_java_version("18.0.2.1+1")).unwrap()),
            "18.0.2.1+1"
        );
        assert_eq!(
            to_java_version(&VersionSpec::parse(from_java_version("21.0.11+10")).unwrap()),
            "21.0.11+10"
        );
    }
}
