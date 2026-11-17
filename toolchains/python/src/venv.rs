// Virtual environments

use moon_config::{UnresolvedVersionSpec, Version};
use moon_pdk_api::MatchesVersion;

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
}
