#![allow(dead_code)]

use proto_pdk::VersionSpec;

pub fn from_swift_version(version: &str) -> String {
    let suffix = match version.matches('.').count() {
        1 => ".0",
        0 => ".0.0",
        _ => "",
    };

    format!("{version}{suffix}")
}

pub fn to_swift_version(spec: &VersionSpec) -> String {
    match spec {
        VersionSpec::Canary => "canary".into(),
        VersionSpec::Alias(alias) => alias.to_string(),
        _ => {
            let version = spec.as_version().unwrap();
            let mut next = version.to_string();

            if let Some(prefix) = next.strip_suffix(".0") {
                next = prefix.to_owned();
            }

            next
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_from_release_tags() {
        assert_eq!(from_swift_version("2.2"), "2.2.0");
        assert_eq!(from_swift_version("6.1.2"), "6.1.2");
    }

    #[test]
    fn formats_to_release_download_version() {
        assert_eq!(
            to_swift_version(&VersionSpec::parse("2.2.0").unwrap()),
            "2.2"
        );
        assert_eq!(
            to_swift_version(&VersionSpec::parse("6.0.0").unwrap()),
            "6.0"
        );
        assert_eq!(
            to_swift_version(&VersionSpec::parse("6.1.2").unwrap()),
            "6.1.2"
        );
    }
}
