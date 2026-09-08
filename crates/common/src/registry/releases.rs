use indexmap::IndexMap;
use proto_pdk::{
    AnyResult, ChecksumAlgorithm, HostArch, HostEnvironment, HostOS, Version, fetch_json,
};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Artifact {
    pub arch: Option<HostArch>,
    pub os: Option<HostOS>,
    pub abi: Option<String>, // TODO
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Release {
    pub version: Version,
    pub builds: IndexMap<String, Build>,
    pub build: Option<Build>,
    pub download_url: String,
    pub checksum_url: Option<String>,
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
    let mut query = String::new();

    if with_filters {
        query = format!("arch={}&os={}", env.arch, env.os);
        // TODO ABI
    }

    let res: VersionsResponse = fetch_json(format!(
        "https://registry.moonrepo.app/releases/{language}/versions?{query}"
    ))?;

    Ok(res.data)
}

pub fn fetch_release(language: &str, version: &Version) -> AnyResult<Release> {
    let res: ReleaseResponse = fetch_json(format!(
        "https://registry.moonrepo.app/releases/{language}/{version}"
    ))?;

    Ok(res.data)
}
