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

/// Extract the version from a `.python-version` file, as pyenv and uv read it: the
/// first line that isn't blank or a comment. Names that aren't a version, like
/// `system` or other implementations (`pypy3.10-7.3.12`), aren't for CPython.
pub fn parse_python_version_file(content: &str) -> Option<String> {
    let version = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))?;

    if version.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }

    // CPython's prerelease format, like `3.15.0rc2`
    let prerelease = Regex::new(r"^([0-9]+\.[0-9]+\.[0-9]+)((?:a|b|rc)[0-9]+)$").unwrap();

    Some(match prerelease.captures(version) {
        Some(caps) => from_python_version(&format!("{}-{}", &caps[1], &caps[2])),
        None => version.to_owned(),
    })
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

    #[test]
    fn parses_version_files() {
        for (content, expected) in [
            ("3.12.1", "3.12.1"),
            ("3.12\n", "3.12"),
            ("3.12.1\n3.11.7\n", "3.12.1"),
            ("# pinned for CI\n\n  3.13 \n", "3.13"),
            (">=3.11,<3.13", ">=3.11,<3.13"),
            ("3.12.1+20240107", "3.12.1+20240107"),
            ("3.15.0rc2", "3.15.0-rc.2"),
            ("3.15.0a1", "3.15.0-alpha.1"),
            ("3.15.0b4", "3.15.0-beta.4"),
        ] {
            assert_eq!(
                parse_python_version_file(content).as_deref(),
                Some(expected),
                "for {content:?}"
            );
        }
    }

    #[test]
    fn ignores_non_cpython_version_files() {
        for content in [
            "system",
            "pypy3.10-7.3.12",
            "miniconda3-latest",
            "# 3.12",
            "",
        ] {
            assert_eq!(parse_python_version_file(content), None, "for {content:?}");
        }
    }
}
