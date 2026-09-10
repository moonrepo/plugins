#![allow(dead_code)]

use proto_pdk_api::Version;

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

// Reverse of the above, and without build metadata
pub fn to_ruby_version(version: &Version) -> String {
    let mut ruby_version = format!("{}.{}.{}", version.major, version.minor, version.patch);

    if let Some(pre) = &version.prerelease {
        ruby_version.push('-');
        ruby_version.push_str(&pre.replace('.', ""));
    }

    ruby_version
}
