// `go.sum`, `go.work.sum`

use moon_config::VersionSpec;
use moon_pdk_api::AnyResult;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
pub struct GoSumDependency {
    pub checksum: String,
    pub version: VersionSpec,
}

#[derive(Debug)]
pub struct GoSum {
    pub dependencies: BTreeMap<String, GoSumDependency>,
}

impl GoSum {
    // https://go.dev/ref/mod#go-sum-files
    pub fn parse(content: impl AsRef<str>) -> AnyResult<Self> {
        let mut sum = Self {
            dependencies: BTreeMap::new(),
        };

        for mut line in content.as_ref().lines() {
            if line.starts_with("//") {
                continue;
            } else if let Some(index) = line.find("//") {
                line = &line[0..index];
            }

            let mut parts = line.trim().splitn(3, ' ');
            let module = parts.next().unwrap_or_default().trim();
            let version = parts.next().unwrap_or_default().trim();
            let hash = parts.next().unwrap_or_default().trim();

            if module.is_empty()
                || version.is_empty()
                || version.ends_with("/go.mod")
                || hash.is_empty()
            {
                continue;
            }

            sum.dependencies.insert(
                module.into(),
                GoSumDependency {
                    checksum: hash.into(),
                    version: VersionSpec::parse(version)?,
                },
            );
        }

        Ok(sum)
    }
}
