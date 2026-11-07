use common::extract_version_from_text;
use proto_pdk_api::{AnyResult, DetectVersionOutput, UnresolvedVersionSpec, Version};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZigVersionSource<'a> {
    Exact(&'a str),
    Minimum(&'a str),
}

impl ZigVersionSource<'_> {
    pub fn to_zig_requirement(self) -> AnyResult<UnresolvedVersionSpec> {
        match self {
            Self::Exact(version) => Ok(UnresolvedVersionSpec::parse(version)?),
            // Zig breaks APIs between minor versions, so stay within
            // the minimum's minor, while allowing newer patches
            Self::Minimum(version) => Ok(UnresolvedVersionSpec::parse(format!("~{version}"))?),
        }
    }

    pub fn to_zls_requirement(self) -> AnyResult<UnresolvedVersionSpec> {
        let raw_version = match self {
            Self::Exact(version) | Self::Minimum(version) => version,
        };

        if matches!(raw_version, "canary" | "master") {
            return Ok(UnresolvedVersionSpec::parse("canary")?);
        }

        // Partial versions and aliases already work as ZLS requirements
        let Ok(version) = Version::parse(raw_version) else {
            return Ok(UnresolvedVersionSpec::parse(raw_version)?);
        };

        if version.prerelease.is_some() {
            return Ok(UnresolvedVersionSpec::parse("canary")?);
        }

        // ZLS releases match Zig's minor versions, but not their patches
        Ok(UnresolvedVersionSpec::parse(format!(
            "~{}.{}",
            version.major, version.minor,
        ))?)
    }
}

pub fn detect_zig_version_files() -> DetectVersionOutput {
    DetectVersionOutput {
        files: vec![
            ".zig-version".into(),
            ".zigversion".into(),
            "build.zig.zon".into(),
        ],
        ignore: vec![".zig-cache".into(), "zig-out".into()],
    }
}

pub fn parse_zig_version_file<'a>(file: &str, content: &'a str) -> Option<ZigVersionSource<'a>> {
    if file == "build.zig.zon" {
        parse_zon_version(content).map(ZigVersionSource::Minimum)
    } else {
        extract_version_from_text(content).map(ZigVersionSource::Exact)
    }
}

fn parse_zon_version(content: &str) -> Option<&str> {
    content.lines().find_map(|line| {
        let line = line.split_once("//").map_or(line, |(code, _)| code);
        let (key, value) = line.split_once('=')?;

        if key.trim() != ".minimum_zig_version" {
            return None;
        }

        let version = value.trim().trim_end_matches(',').trim();
        version.strip_prefix('"')?.strip_suffix('"')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_zig_version_sources() {
        assert_eq!(
            parse_zig_version_file(".zig-version", "\n0.15.2\n"),
            Some(ZigVersionSource::Exact("0.15.2"))
        );
        assert_eq!(
            parse_zig_version_file(".zig-version", "# pinned for CI\n0.15.2 # stable\n"),
            Some(ZigVersionSource::Exact("0.15.2"))
        );
        assert_eq!(
            parse_zig_version_file(
                "build.zig.zon",
                r#".{
                    .minimum_zig_version = "0.15.2", // Required for APIs.
                }"#,
            ),
            Some(ZigVersionSource::Minimum("0.15.2"))
        );
    }

    #[test]
    fn creates_zig_requirements() {
        assert_eq!(
            ZigVersionSource::Exact("0.15.2")
                .to_zig_requirement()
                .unwrap()
                .to_string(),
            "0.15.2"
        );
        assert_eq!(
            ZigVersionSource::Minimum("0.15.2")
                .to_zig_requirement()
                .unwrap()
                .to_string(),
            "~0.15.2"
        );
    }

    #[test]
    fn creates_zls_requirements() {
        assert_eq!(
            ZigVersionSource::Exact("0.15.2")
                .to_zls_requirement()
                .unwrap()
                .to_string(),
            "~0.15"
        );
        assert_eq!(
            ZigVersionSource::Minimum("0.15.2")
                .to_zls_requirement()
                .unwrap()
                .to_string(),
            "~0.15"
        );
        assert_eq!(
            ZigVersionSource::Exact("0.15")
                .to_zls_requirement()
                .unwrap()
                .to_string(),
            "~0.15"
        );
        assert_eq!(
            ZigVersionSource::Exact("latest")
                .to_zls_requirement()
                .unwrap()
                .to_string(),
            "latest"
        );
        assert_eq!(
            ZigVersionSource::Exact("0.16.0-dev.123+abc")
                .to_zls_requirement()
                .unwrap()
                .to_string(),
            "canary"
        );
    }
}
