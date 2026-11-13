use proto_pdk_api::{Version, VersionSpec};

/// Convert a Java version to semver, by padding the minor and patch,
/// for example `21` to `21.0.0`. A 4th component is kept as the revision.
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

    // A 4th "vendor" component (18.0.2.1) is the revision. A 0 component
    // (21.0.13.0, as SDKMAN writes) is the same version, so is dropped.
    if let Some(revision) = parts.next().filter(|revision| *revision != "0") {
        out.push('.');
        out.push_str(revision);
    }

    if let Some(pre) = pre {
        out.push('-');
        out.push_str(pre);
    }

    if let Some(build) = build {
        out.push('+');
        out.push_str(build);
    }

    out
}

/// Before proto supported revisions, a 4th component was encoded into the
/// pre-release as `vN`, for example `18.0.2.1` as `18.0.2-v1` (or `18.0.2-ea.v1`
/// with a pre-release). Decode it into the revision, for existing pins.
pub fn decode_legacy_revision(version: &Version) -> Option<Version> {
    fn is_digits(value: &str) -> bool {
        !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
    }

    let pre = version.prerelease.as_deref()?;
    let (pre, revision) = match pre.split_once(".v") {
        Some((head, tail)) if is_digits(tail) => (Some(head), tail),
        _ => (None, pre.strip_prefix('v').filter(|tail| is_digits(tail))?),
    };

    let mut version = version.to_owned();
    version.revision = Some(revision.parse().ok()?);
    version.prerelease = pre.map(|pre| pre.into());

    Some(version)
}

pub fn to_java_version(spec: &VersionSpec) -> String {
    match spec {
        VersionSpec::Canary => "canary".into(),
        VersionSpec::Alias(alias) => alias.to_string(),
        _ => {
            let version = spec.as_version().unwrap();
            let decoded = decode_legacy_revision(version);
            let version = decoded.as_ref().unwrap_or(version);

            // A 0 revision is the same version (see `from_java_version`)
            let revision = version.revision.filter(|revision| *revision > 0);

            let mut out = version.major.to_string();

            if version.minor > 0 || version.patch > 0 || revision.is_some() {
                out.push('.');
                out.push_str(&version.minor.to_string());

                if version.patch > 0 || revision.is_some() {
                    out.push('.');
                    out.push_str(&version.patch.to_string());
                }
            }

            if let Some(revision) = revision {
                out.push('.');
                out.push_str(&revision.to_string());
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
        assert_eq!(from_java_version("1.2.3.4"), "1.2.3.4");
        assert_eq!(from_java_version("1.2.3.0"), "1.2.3");

        assert_eq!(from_java_version("1-ea"), "1.0.0-ea");
        assert_eq!(from_java_version("1.2-ea"), "1.2.0-ea");
        assert_eq!(from_java_version("1.2.3-ea"), "1.2.3-ea");
        assert_eq!(from_java_version("1.2.3.4-ea"), "1.2.3.4-ea");

        assert_eq!(from_java_version("1+1"), "1.0.0+1");
        assert_eq!(from_java_version("1.2+2"), "1.2.0+2");
        assert_eq!(from_java_version("1.2.3+3"), "1.2.3+3");
        assert_eq!(from_java_version("1.2.3.4+4"), "1.2.3.4+4");

        assert_eq!(from_java_version("1-ea+1"), "1.0.0-ea+1");
        assert_eq!(from_java_version("1.2-ea+2"), "1.2.0-ea+2");
        assert_eq!(from_java_version("1.2.3-ea+3"), "1.2.3-ea+3");
        assert_eq!(from_java_version("1.2.3.4-ea+4"), "1.2.3.4-ea+4");

        // Shouldn't change
        assert_eq!(from_java_version("1.0.0"), "1.0.0");
        assert_eq!(from_java_version("1.0.0-alpha1"), "1.0.0-alpha1");

        // Real-world versions
        assert_eq!(from_java_version("21.0.11+10"), "21.0.11+10");
        assert_eq!(from_java_version("8.0.492+9"), "8.0.492+9");
        assert_eq!(from_java_version("18.0.2.1+1"), "18.0.2.1+1");
        assert_eq!(from_java_version("21.0.13.0"), "21.0.13");
    }

    #[test]
    fn decodes_legacy_revisions() {
        let decode = |version: &str| {
            decode_legacy_revision(&Version::parse(version).unwrap())
                .map(|version| version.to_string())
        };

        assert_eq!(decode("1.2.3-v4").as_deref(), Some("1.2.3.4"));
        assert_eq!(decode("1.2.3-v4+4").as_deref(), Some("1.2.3.4+4"));
        assert_eq!(decode("1.2.3-ea.v4+4").as_deref(), Some("1.2.3.4-ea+4"));
        assert_eq!(decode("18.0.2-v1+1").as_deref(), Some("18.0.2.1+1"));

        // Not encoded
        assert_eq!(decode("1.2.3"), None);
        assert_eq!(decode("1.2.3-ea"), None);
        assert_eq!(decode("1.2.3-vendor"), None);
        assert_eq!(decode("1.2.3-ea.v"), None);
    }

    #[test]
    fn formats_to() {
        let to = |version: &str| to_java_version(&VersionSpec::parse(version).unwrap());

        assert_eq!(to("1.0.0"), "1");
        assert_eq!(to("1.0.1"), "1.0.1");
        assert_eq!(to("1.2.0"), "1.2");
        assert_eq!(to("1.2.3"), "1.2.3");
        assert_eq!(to("1.2.3.4"), "1.2.3.4");
        assert_eq!(to("1.0.0.4"), "1.0.0.4");
        assert_eq!(to("1.2.3.0"), "1.2.3");

        assert_eq!(to("1.0.0+1"), "1+1");
        assert_eq!(to("1.2.0+2"), "1.2+2");
        assert_eq!(to("1.2.3+3"), "1.2.3+3");
        assert_eq!(to("1.2.3.4+4"), "1.2.3.4+4");

        assert_eq!(to("1.0.0-ea+1"), "1-ea+1");
        assert_eq!(to("1.2.0-ea+2"), "1.2-ea+2");
        assert_eq!(to("1.2.3-ea+3"), "1.2.3-ea+3");
        assert_eq!(to("1.2.3.4-ea+4"), "1.2.3.4-ea+4");

        // Legacy revisions
        assert_eq!(to("1.2.3-v4"), "1.2.3.4");
        assert_eq!(to("1.2.3-v4+4"), "1.2.3.4+4");
        assert_eq!(to("1.2.3-ea.v4+4"), "1.2.3.4-ea+4");
        assert_eq!(to("18.0.2-v1+1"), "18.0.2.1+1");
    }

    #[test]
    fn round_trips() {
        for version in ["18.0.2.1+1", "21.0.11+10", "1.2.3.4-ea+4"] {
            assert_eq!(
                to_java_version(&VersionSpec::parse(from_java_version(version)).unwrap()),
                version
            );
        }
    }
}
