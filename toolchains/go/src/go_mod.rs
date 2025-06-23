// `go.mod`

use go_tool::version::from_go_version;
use moon_pdk_api::{AnyResult, anyhow};
use std::str::FromStr;

pub use gomod_parser2::*;

pub fn parse_go_mod(content: impl AsRef<str>) -> AnyResult<GoMod> {
    let mut go_mod = GoMod::from_str(content.as_ref()).map_err(|error| anyhow!("{error}"))?;

    go_mod.go = go_mod.go.map(|version| from_go_version(&version));

    Ok(go_mod)
}
