mod bun;
mod deno;
mod npm;
mod pnpm;
mod yarn;

pub use bun::*;
pub use deno::*;
pub use npm::*;
pub use pnpm::*;
pub use yarn::*;

use moon_pdk::AnyResult;
use moon_pdk_api::VersionSpec;

pub(super) fn parse_version_spec<T: AsRef<str>>(version: T) -> AnyResult<Option<VersionSpec>> {
    let version = version.as_ref();

    if version.is_empty() || version.contains(':') {
        Ok(None)
    } else {
        Ok(Some(VersionSpec::parse(version)?))
    }
}

// pub(super) fn parse_version<T: AsRef<str>>(version: T) -> AnyResult<Option<Version>> {
//     Ok(parse_version_spec(version)?.and_then(|spec| spec.as_version().cloned()))
// }
