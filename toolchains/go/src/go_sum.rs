// `go.sum`, `go.work.sum`

use moon_config::VersionSpec;
use moon_pdk::AnyResult;
use std::collections::BTreeMap;

pub struct GoSumDependency {
    pub checksum: String,
    pub version: VersionSpec,
}

pub struct GoSum {
    pub dependencies: BTreeMap<String, GoSumDependency>,
}

impl GoSum {
    // https://go.dev/ref/mod#go-sum-files
    pub fn parse(content: impl AsRef<str>) -> AnyResult<Self> {
        let mut sum = Self {
            dependencies: BTreeMap::new(),
        };

        for line in content.as_ref().lines() {
            let mut parts = line.splitn(3, ' ');
            let module = parts.next().unwrap_or_default();
            let version = parts.next().unwrap_or_default();
            let hash = parts.next().unwrap_or_default();

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
