#![allow(dead_code)]

use proto_pdk_api::{Version, extract_version_from_text};

// Ruby tags, ruby-build, and GitHub releases use `preview2` and `rc1`,
// while the registry (and semver ordering) use `preview.2` and `rc.1`
pub fn from_ruby_version(version: &str) -> String {
    if let Some((base, pre)) = version.split_once('-')
        && !pre.contains('.')
        && let Some(index) = pre.find(|c: char| c.is_ascii_digit())
        && index > 0
    {
        return format!("{base}-{}.{}", &pre[..index], &pre[index..]);
    }

    version.to_owned()
}

/// Extract the CRuby version from a `.ruby-version` file. Version managers accept
/// an optional `ruby-` prefix, while anything else that isn't a version, like
/// `jruby-9.4.5.0` or `system`, is for a different engine or manager.
pub fn parse_ruby_version_file(content: &str) -> Option<&str> {
    let version = extract_version_from_text(content)?
        .split_whitespace()
        .next()?;
    let version = version.strip_prefix("ruby-").unwrap_or(version);

    version
        .starts_with(|c: char| c.is_ascii_digit())
        .then_some(version)
}

// Reverse of the above, and without build metadata
pub fn to_ruby_version(version: &Version) -> String {
    let mut ruby_version = format!("{}.{}.{}", version.major, version.minor, version.patch);

    if let Some(pre) = &version.prerelease {
        ruby_version.push('-');
        ruby_version.push_str(&pre.replace('.', ""));
    }

    ruby_version
}
