use indexmap::IndexMap;
use proto_pdk::{
    AnyResult, ChecksumAlgorithm, DownloadPrebuiltOutput, HostArch, HostEnvironment, HostLibc,
    HostOS, Version, VersionSpec, fetch_json,
};
use serde::{Deserialize, Deserializer};

// Some artifacts use a libc/ABI that `HostLibc` doesn't support (like `msvc`),
// so ignore those values instead of failing, which allows the artifact
// to be matched against any host
fn deserialize_libc<'de, D: Deserializer<'de>>(des: D) -> Result<Option<HostLibc>, D::Error> {
    Ok(match Option::<String>::deserialize(des)?.as_deref() {
        Some("gnu") => Some(HostLibc::Gnu),
        Some("musl") => Some(HostLibc::Musl),
        _ => None,
    })
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Artifact {
    pub arch: Option<HostArch>,
    pub os: Option<HostOS>,
    #[serde(deserialize_with = "deserialize_libc")]
    pub libc: Option<HostLibc>,
    pub abi: Option<String>,
    pub archive_file: String,
    pub size: Option<i64>,
    pub checksum_file: Option<String>,
    pub checksum_type: Option<ChecksumAlgorithm>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Build {
    pub tag: String,
    pub artifacts: IndexMap<String, Artifact>,
    pub artifact: Option<Artifact>,
    pub checksums_file: Option<String>,
    pub checksums_type: Option<ChecksumAlgorithm>,
}

impl Build {
    pub fn get_artifact(&self, env: &HostEnvironment) -> Option<&Artifact> {
        self.artifacts
            .values()
            .find(|art| {
                art.arch.as_ref().is_none_or(|arch| arch == &env.arch)
                    && art.os.as_ref().is_none_or(|os| os == &env.os)
                    && art.libc.as_ref().is_none_or(|libc| libc == &env.libc)
            })
            .or(self.artifact.as_ref())
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Release {
    pub version: Version,
    pub builds: IndexMap<String, Build>,
    pub build: Option<Build>,
    pub download_url: String,
    pub checksum_url: Option<String>,
}

impl Release {
    pub fn get_build(&self, version: &Version) -> Option<&Build> {
        if let Some(build_id) = &version.build
            && let Some(build) = self.builds.get(build_id.as_str())
        {
            return Some(build);
        }

        self.build.as_ref()
    }

    pub fn create_download_prebuilt(
        &self,
        env: &HostEnvironment,
        version: &Version,
    ) -> Option<DownloadPrebuiltOutput> {
        let build = self.get_build(version)?;
        let artifact = build.get_artifact(env)?;

        let mut output = DownloadPrebuiltOutput {
            download_name: Some(artifact.archive_file.clone()),
            download_url: self
                .download_url
                .replace("{tag}", &build.tag)
                .replace("{file}", &artifact.archive_file),
            ..Default::default()
        };

        if let Some(checksum_file) = artifact
            .checksum_file
            .as_ref()
            .or(build.checksums_file.as_ref())
        {
            output.checksum_name = Some(checksum_file.into());
            output.checksum_url = Some(
                self.checksum_url
                    .as_ref()
                    .unwrap_or(&self.download_url)
                    .replace("{tag}", &build.tag)
                    .replace("{file}", checksum_file),
            );
        }

        Some(output)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct VersionsResponse {
    pub data: Vec<Version>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReleaseResponse {
    pub data: Release,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ReleasesResponse {
    pub data: IndexMap<Version, Release>,
}

pub fn fetch_versions(
    env: &HostEnvironment,
    language: &str,
    with_filters: bool,
) -> AnyResult<Vec<VersionSpec>> {
    let mut query = vec![];

    if with_filters {
        query.push(format!("arch={}", env.arch.to_rust_arch()));
        query.push(format!("os={}", env.os.to_rust_os()));

        // macOS reports a gnu libc, but those artifacts have no libc at
        // all, so filtering by it would return zero results
        if env.os == HostOS::Linux && matches!(env.libc, HostLibc::Gnu | HostLibc::Musl) {
            query.push(format!("libc={}", env.libc));
        }
    }

    let res: VersionsResponse = fetch_json(format!(
        "https://registry.moonrepo.app/releases/{language}/versions?{}",
        query.join("&")
    ))?;

    Ok(res.data.into_iter().map(VersionSpec::Version).collect())
}

pub fn fetch_release(language: &str, version: &Version) -> AnyResult<Release> {
    let res: ReleaseResponse = fetch_json(format!(
        "https://registry.moonrepo.app/releases/{language}/{version}"
    ))?;

    Ok(res.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_build(libc: &str) -> Build {
        serde_json::from_str(&format!(
            r#"{{"artifacts":{{"triple":{{"arch":"x86_64","os":"windows",{libc}"archive_file":"a.tar.gz"}}}}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn ignores_unsupported_libc() {
        assert_eq!(
            create_build(r#""libc":"msvc","#).artifacts["triple"].libc,
            None
        );
        assert_eq!(create_build("").artifacts["triple"].libc, None);
        assert_eq!(
            create_build(r#""libc":"musl","#).artifacts["triple"].libc,
            Some(HostLibc::Musl)
        );
    }

    #[test]
    fn matches_artifact_ignoring_unsupported_libc() {
        let build = create_build(r#""libc":"msvc","#);
        let env = HostEnvironment {
            arch: HostArch::X64,
            os: HostOS::Windows,
            libc: HostLibc::Unknown,
            ..Default::default()
        };

        assert!(build.get_artifact(&env).is_some());
    }
}
