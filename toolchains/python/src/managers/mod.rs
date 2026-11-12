mod pip;
mod poetry;
mod uv;

pub use pip::*;
pub use poetry::*;
pub use uv::*;

use extism_pdk::debug;
use moon_config::VersionSpec;
use pep440_rs::Version;
use std::str::FromStr;

// ADR-0008.5: Skip a version that can't be parsed, instead of failing the whole lockfile
pub(super) fn parse_version_spec<T: AsRef<str>>(name: &str, version: T) -> Option<VersionSpec> {
    let version = version.as_ref();

    if version.is_empty() || version.contains(':') {
        return None;
    }

    let value = match Version::from_str(version) {
        Ok(value) => value,
        Err(error) => {
            debug!("Skipping the locked version {version} of {name}: {error}");

            return None;
        }
    };

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

    match VersionSpec::parse(&spec) {
        Ok(spec) => Some(spec),
        Err(error) => {
            debug!("Skipping the locked version {version} of {name}: {error}");

            None
        }
    }
}
