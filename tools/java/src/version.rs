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

    if let Some(pre) = pre {
        out.push('-');
        out.push_str(pre);
    }

    if let Some(build) = build {
        out.push('+');
        out.push_str(build);
    }

    match parts.next() {
        Some(vendor) => format!("{vendor}-{out}"),
        None => out,
    }
}

pub fn to_java_version(spec: &VersionSpec) -> String {
    match spec {
        VersionSpec::Canary => "canary".into(),
        VersionSpec::Alias(alias) => alias.to_string(),
        _ => {
            let version = spec.as_version().unwrap();
            let mut out = version.major.to_string();

            if version.minor > 0 || version.patch > 0 {
                out.push('.');
                out.push_str(&version.minor.to_string());

                if version.patch > 0 {
                    out.push('.');
                    out.push_str(&version.patch.to_string());
                }
            }

            if let Some(pre) = &version.prerelease {
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
    }
}
