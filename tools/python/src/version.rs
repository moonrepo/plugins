#![allow(dead_code)]

use proto_pdk_api::Version;
use regex::Regex;

// Python tags, python-build, and the pre-built archives use `3.15.0rc2`,
// while the registry (and semver ordering) uses `3.15.0-rc.2`
pub fn from_python_version(version: &str) -> String {
    let Some((base, pre)) = version.split_once('-') else {
        return version.to_owned();
    };

    let Some(index) = pre.find(|c: char| c.is_ascii_digit()) else {
        return version.to_owned();
    };

    let (name, id) = pre.split_at(index);

    let name = match name.trim_end_matches('.') {
        "a" | "alpha" => "alpha",
        "b" | "beta" => "beta",
        "c" | "rc" => "rc",
        _ => return version.to_owned(),
    };

    format!("{base}-{name}.{id}")
}

// Reverse of the above, and without build metadata
pub fn to_python_version(version: &Version) -> String {
    let mut python_version = format!("{}.{}.{}", version.major, version.minor, version.patch);

    if let Some(pre) = &version.prerelease {
        let (name, id) = pre.split_once('.').unwrap_or((pre.as_str(), ""));

        python_version.push_str(match name {
            "alpha" => "a",
            "beta" => "b",
            other => other,
        });
        python_version.push_str(id);
    }

    python_version
}

pub fn from_python_tag(tag: String, regex: &Regex) -> Option<String> {
    let caps = regex.captures(&tag)?;

    let mut version = format!(
        "{}.{}.{}",
        &caps["major"],
        &caps["minor"],
        caps.name("patch").map(|c| c.as_str()).unwrap_or("0"),
    );

    if let Some(pre) = caps.name("pre") {
        version.push_str(&format!("-{}{}", pre.as_str(), &caps["preid"]));
        version = from_python_version(&version);
    }

    Some(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_regex() -> Regex {
        Regex::new(
            r"v?(?<major>[0-9]+)\.(?<minor>[0-9]+)(?:\.(?<patch>[0-9]+))?(?:(?<pre>a|b|c|rc)(?<preid>[0-9]+))?",
        )
        .unwrap()
    }

    #[test]
    fn converts_tags_to_registry_versions() {
        let regex = create_regex();
        let convert = |tag: &str| from_python_tag(tag.into(), &regex).unwrap();

        assert_eq!(convert("v3.14.2"), "3.14.2");
        assert_eq!(convert("v3.15"), "3.15.0");
        assert_eq!(convert("v3.15.0a1"), "3.15.0-alpha.1");
        assert_eq!(convert("v3.15.0b4"), "3.15.0-beta.4");
        assert_eq!(convert("v3.15.0rc2"), "3.15.0-rc.2");
        assert_eq!(convert("v2.6.0c1"), "2.6.0-rc.1");
    }

    #[test]
    fn normalizes_short_and_undotted_prereleases() {
        assert_eq!(from_python_version("3.15.0-rc2"), "3.15.0-rc.2");
        assert_eq!(from_python_version("3.15.0-a.1"), "3.15.0-alpha.1");
        assert_eq!(from_python_version("3.15.0-b.4"), "3.15.0-beta.4");
        assert_eq!(from_python_version("3.15.0-alpha.1"), "3.15.0-alpha.1");
        assert_eq!(
            from_python_version("3.15.0-rc2+20260901"),
            "3.15.0-rc.2+20260901"
        );
    }

    #[test]
    fn leaves_other_versions_alone() {
        assert_eq!(from_python_version("3.14.2"), "3.14.2");
        assert_eq!(from_python_version("3.14.2+20260901"), "3.14.2+20260901");
        assert_eq!(from_python_version("3.15.0-dev"), "3.15.0-dev");
        assert_eq!(from_python_version("3.15.0-1"), "3.15.0-1");
    }

    #[test]
    fn converts_back_to_python_versions() {
        let convert = |version: &str| to_python_version(&Version::parse(version).unwrap());

        assert_eq!(convert("3.14.2"), "3.14.2");
        assert_eq!(convert("3.14.2+20260901"), "3.14.2");
        assert_eq!(convert("3.15.0-alpha.1"), "3.15.0a1");
        assert_eq!(convert("3.15.0-beta.4"), "3.15.0b4");
        assert_eq!(convert("3.15.0-rc.2"), "3.15.0rc2");
        assert_eq!(convert("3.15.0-rc.2+20260901"), "3.15.0rc2");
    }
}
