/// Convert a PEP 440 version (a Git tag) to semver, like `1.2.0rc1` to `1.2.0-rc1`.
pub fn from_pep_version(version: &str) -> String {
    for del in ["a", "b", "rc"] {
        if let Some(parts) = version.split_once(del) {
            return format!("{}-{}{}", parts.0, del, parts.1);
        }
    }

    version.to_owned()
}

/// Convert a semver version back to PEP 440, like `1.2.0-rc1` to `1.2.0rc1`,
/// as the installer only accepts versions as they're published on PyPI.
pub fn to_pep_version(version: &str) -> String {
    version.replace('-', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_from_pep() {
        assert_eq!(from_pep_version("2.1.0"), "2.1.0");
        assert_eq!(from_pep_version("1.0.0a5"), "1.0.0-a5");
        assert_eq!(from_pep_version("1.1.0b4"), "1.1.0-b4");
        assert_eq!(from_pep_version("1.2.0rc2"), "1.2.0-rc2");
    }

    #[test]
    fn converts_to_pep() {
        assert_eq!(to_pep_version("2.1.0"), "2.1.0");
        assert_eq!(to_pep_version("1.0.0-a5"), "1.0.0a5");
        assert_eq!(to_pep_version("1.1.0-b4"), "1.1.0b4");
        assert_eq!(to_pep_version("1.2.0-rc2"), "1.2.0rc2");
    }
}
