use indexmap::IndexMap;
use proto_pdk::{
    AnyResult, ChecksumAlgorithm, DownloadPrebuiltOutput, HostArch, HostEnvironment, HostLibc,
    HostOS, Version, fetch_json,
};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Artifact {
    pub arch: Option<HostArch>,
    pub os: Option<HostOS>,
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
        id: String,
        env: &HostEnvironment,
        version: &Version,
    ) -> Option<DownloadPrebuiltOutput> {
        let build = self.get_build(version)?;
        let artifact = build.get_artifact(env)?;

        let mut output = DownloadPrebuiltOutput {
            download_name: Some(artifact.archive_file.clone()),
            download_url: self
                .download_url
                .replace("{release}", &id)
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
                    .replace("{release}", &id)
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
) -> AnyResult<Vec<Version>> {
    let mut query = vec![];

    if with_filters {
        query.push(format!("arch={}", env.arch.to_rust_arch()));
        query.push(format!("os={}", env.os.to_rust_os()));

        if matches!(env.libc, HostLibc::Gnu | HostLibc::Musl) {
            query.push(format!("libc={}", env.libc));
        }
    }

    let res: VersionsResponse = fetch_json(format!(
        "https://registry.moonrepo.app/releases/{language}/versions?{}",
        query.join("&")
    ))?;

    Ok(res.data)
}

pub fn fetch_release(language: &str, version: &Version) -> AnyResult<Release> {
    let res: ReleaseResponse = fetch_json(format!(
        "https://registry.moonrepo.app/releases/{language}/{version}"
    ))?;

    Ok(res.data)
}
