// `go.mod`

use moon_pdk::{AnyResult, anyhow};
use std::str::FromStr;

pub use gomod_parser::*;

pub fn parse_go_mod(content: impl AsRef<str>) -> AnyResult<GoMod> {
    GoMod::from_str(content.as_ref()).map_err(|error| anyhow!("{error}"))
}
