// `go.sum`, `go.work.sum`

use crate::parsers;
use moon_config::VersionSpec;
use moon_pdk_api::AnyResult;
use std::collections::BTreeMap;
use toolchain_common::debug;

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

        for entry in parsers::go_sum::parse(content.as_ref()) {
            // Checksums of a version's `go.mod` file only
            if entry.version.ends_with("/go.mod") {
                continue;
            }

            // ADR-0008.5: Skip a version that can't be parsed, instead of failing the whole file
            let version = match VersionSpec::parse(entry.version) {
                Ok(version) => version,
                Err(error) => {
                    debug!(
                        "Skipping the version {} of {} in go.sum: {error}",
                        entry.version, entry.module
                    );

                    continue;
                }
            };

            sum.dependencies
                .entry(entry.module.into())
                .or_default()
                .push(GoSumDependency {
                    checksum: entry.checksum.into(),
                    version,
                });
        }

        Ok(sum)
    }
}
