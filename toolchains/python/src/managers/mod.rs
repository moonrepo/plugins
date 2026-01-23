mod pip;
mod uv;

pub use pip::*;
pub use uv::*;

use moon_config::VersionSpec;
use moon_pdk::AnyResult;
use pep440_rs::Version;
use std::str::FromStr;

pub(super) fn parse_version_spec<T: AsRef<str>>(version: T) -> AnyResult<Option<VersionSpec>> {
    let version = version.as_ref();

    if version.is_empty() || version.contains(':') {
        return Ok(None);
    }

    if let Ok(value) = Version::from_str(version) {
        let mut parts = value.release().to_vec();

        if parts.len() > 3 {
            parts.truncate(3);
        } else {
            while parts.len() < 3 {
                parts.push(0);
            }
        }

        let mut spec = parts
            .into_iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(".");
        let mut meta = vec![];

        // Based on https://docs.rs/pep440_rs/latest/src/pep440_rs/version.rs.html#686
        // but modified to our version format
        if let Some(pre) = value.pre() {
            meta.push(format!("{}{}", pre.kind, pre.number));
        }

        if let Some(post) = value.post() {
            meta.push(format!("post{post}"));
        }

        if let Some(dev) = value.dev() {
            meta.push(format!("dev{dev}"));
        }

        for local in value.local() {
            meta.push(local.to_string());
        }

        if !meta.is_empty() {
            spec.push('+');
            spec.push_str(&meta.join("."));
        }

        return Ok(Some(VersionSpec::parse(spec)?));
    }

    Ok(None)
}
