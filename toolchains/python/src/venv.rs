// Virtual environments

use moon_config::{UnresolvedVersionSpec, Version};
use moon_pdk_api::MatchesVersion;
use proto_pdk_api::Op;
use python_tool::to_python_version;
use std::str::FromStr;

/// The version of pip from the name of its `pip-<version>.dist-info` directory.
pub fn parse_pip_dist_info_version(name: &str) -> Option<pep440_rs::Version> {
    let version = name.strip_prefix("pip-")?.strip_suffix(".dist-info")?;

    pep440_rs::Version::from_str(version).ok()
}

/// Whether pip supports `--group` (PEP 735), which pip 25.1 added.
pub fn pip_supports_dependency_groups(version: &pep440_rs::Version) -> bool {
    version.release() >= [25, 1].as_slice()
}

/// The Python request for `UV_PYTHON`, which uv reads as a version or a
/// partial version. Only a spec without a range maps to one, like `3.14.6`,
/// `3.14` or `3.14.*`, so `^3.12`, `>=3.11`, ranges and aliases return `None`.
pub fn to_uv_python_request(spec: &UnresolvedVersionSpec) -> Option<String> {
    match spec {
        // Without build metadata, and pre-releases like `3.15.0rc2`
        UnresolvedVersionSpec::Version(version) => Some(to_python_version(version)),
        // `~3.14.2` is a range of patches, but `~3.14` and `3.14` aren't
        UnresolvedVersionSpec::Requirement(req)
            if req.major.is_some()
                && (req.op == Op::Exact
                    || (matches!(req.op, Op::Tilde | Op::Wildcard) && req.patch.is_none())) =>
        {
            Some(spec.to_partial_string())
        }
        _ => None,
    }
}

/// The Python version that a venv was created with, from its `pyvenv.cfg`.
/// The stdlib writes it as `version`, and uv as `version_info`.
pub fn parse_pyvenv_cfg_version(content: &str) -> Option<Version> {
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if matches!(key.trim(), "version" | "version_info") {
            // Only the release, like `3.15.0` from `3.15.0rc2`
            let release = value
                .trim()
                .split(|ch: char| !ch.is_ascii_digit() && ch != '.')
                .next()?;

            return Version::parse(release).ok();
        }
    }

    None
}

/// Whether a venv's Python satisfies the configured version. Build metadata
/// and pre-releases are ignored, as the venv only records the release.
/// Aliases can't be compared, so they always match.
pub fn is_venv_version_compatible(venv_version: &Version, spec: &UnresolvedVersionSpec) -> bool {
    match spec {
        UnresolvedVersionSpec::Version(version) => {
            version.major == venv_version.major
                && version.minor == venv_version.minor
                && version.patch == venv_version.patch
        }
        UnresolvedVersionSpec::Requirement(req) => req.matches(venv_version),
        UnresolvedVersionSpec::Range(range) => range.matches(venv_version),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(value: &str) -> Version {
        Version::parse(value).unwrap()
    }

    fn spec(value: &str) -> UnresolvedVersionSpec {
        UnresolvedVersionSpec::parse(value).unwrap()
    }

    #[test]
    fn parses_stdlib_and_uv_keys() {
        assert_eq!(
            parse_pyvenv_cfg_version("home = /bin\nversion = 3.14.6\nexecutable = /bin/python"),
            Some(version("3.14.6"))
        );
        assert_eq!(
            parse_pyvenv_cfg_version(
                "home = /bin\nimplementation = CPython\nversion_info = 3.13.2"
            ),
            Some(version("3.13.2"))
        );
        assert_eq!(
            parse_pyvenv_cfg_version("version_info = 3.15.0rc2"),
            Some(version("3.15.0"))
        );
    }

    #[test]
    fn skips_missing_or_invalid_versions() {
        assert_eq!(parse_pyvenv_cfg_version("home = /bin"), None);
        assert_eq!(parse_pyvenv_cfg_version("version = unknown"), None);
    }

    #[test]
    fn compares_versions() {
        let venv = version("3.14.6");

        assert!(is_venv_version_compatible(&venv, &spec("3.14.6")));
        assert!(is_venv_version_compatible(&venv, &spec("3.14.6+20260101")));
        assert!(!is_venv_version_compatible(&venv, &spec("3.14.7")));
        assert!(is_venv_version_compatible(&venv, &spec("3.14")));
        assert!(!is_venv_version_compatible(&venv, &spec("3.13")));
        assert!(is_venv_version_compatible(&venv, &spec("^3.12")));
        assert!(is_venv_version_compatible(&venv, &spec("latest")));
    }

    #[test]
    fn parses_pip_dist_info_versions() {
        let pip = |name: &str| parse_pip_dist_info_version(name).map(|v| v.to_string());

        assert_eq!(pip("pip-25.1.dist-info"), Some("25.1".into()));
        assert_eq!(pip("pip-24.0.dist-info"), Some("24.0".into()));
        assert_eq!(pip("pip-26.1.2.dist-info"), Some("26.1.2".into()));
        assert_eq!(pip("pip-tools-7.4.1.dist-info"), None);
        assert_eq!(pip("pip"), None);
        assert_eq!(pip("six-1.17.0.dist-info"), None);
    }

    #[test]
    fn checks_pip_dependency_group_support() {
        let supports = |value: &str| {
            pip_supports_dependency_groups(&pep440_rs::Version::from_str(value).unwrap())
        };

        assert!(supports("25.1"));
        assert!(supports("25.1.1"));
        assert!(supports("26.0"));
        assert!(!supports("25.0.1"));
        assert!(!supports("24.3.1"));
    }

    #[test]
    fn converts_specs_without_a_range_to_uv_requests() {
        let request = |value: &str| to_uv_python_request(&spec(value));

        assert_eq!(request("3.14.6"), Some("3.14.6".into()));
        assert_eq!(request("3.14.6+20260101"), Some("3.14.6".into()));
        assert_eq!(request("3.15.0-rc.2"), Some("3.15.0rc2".into()));
        assert_eq!(request("3.14"), Some("3.14".into()));
        assert_eq!(request("~3.14"), Some("3.14".into()));
        assert_eq!(request("3.14.*"), Some("3.14".into()));
        assert_eq!(request("=3.14.6"), Some("3.14.6".into()));
        assert_eq!(request("3"), Some("3".into()));
    }

    #[test]
    fn skips_specs_with_a_range() {
        let request = |value: &str| to_uv_python_request(&spec(value));

        assert_eq!(request("^3.12"), None);
        assert_eq!(request(">=3.11"), None);
        assert_eq!(request("~3.14.2"), None);
        assert_eq!(request(">=3.11 <3.14"), None);
        assert_eq!(request("*"), None);
        assert_eq!(request("latest"), None);
        assert_eq!(request("stable"), None);
    }
}
