// `go.sum`, `go.work.sum`

use crate::syntax::{field, line_end, parse_entries, ws, ws1};
use moon_config::VersionSpec;
use moon_pdk_api::AnyResult;
use std::collections::BTreeMap;
use winnow::Result;
use winnow::combinator::alt;
use winnow::prelude::*;

#[derive(Debug, PartialEq)]
pub struct GoSumDependency {
    pub checksum: String,
    pub version: VersionSpec,
}

#[derive(Debug)]
pub struct GoSum {
    /// Every version listed for a module, as an untidied
    /// `go.sum` keeps checksums for older versions
    pub dependencies: BTreeMap<String, Vec<GoSumDependency>>,
}

impl GoSum {
    // https://go.dev/ref/mod#go-sum-files
    pub fn parse(content: impl AsRef<str>) -> AnyResult<Self> {
        let mut sum = Self {
            dependencies: BTreeMap::new(),
        };

        for (module, version, checksum) in
            parse_entries(content.as_ref(), line).into_iter().flatten()
        {
            // Checksums of a version's `go.mod` file only
            if version.ends_with("/go.mod") {
                continue;
            }

            sum.dependencies
                .entry(module.into())
                .or_default()
                .push(GoSumDependency {
                    checksum: checksum.into(),
                    version: VersionSpec::parse(version)?,
                });
        }

        Ok(sum)
    }
}

/// `<module> <version>[/go.mod] <checksum>`, or a blank line. Go doesn't
/// write comments, but they're allowed like in its other files.
fn line<'i>(input: &mut &'i str) -> Result<Option<(&'i str, &'i str, &'i str)>> {
    alt((
        line_end.map(|()| None),
        (ws, field, ws1, field, ws1, field, line_end)
            .map(|(_, module, _, version, _, checksum, _)| Some((module, version, checksum))),
    ))
    .parse_next(input)
}
