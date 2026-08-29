use super::VERSION_REGEX;
use proto_pdk::{
    DetectVersionOutput, DownloadPrebuiltOutput, HostArch, HostLibc, HostOS, LoadVersionsOutput,
    LocateExecutablesOutput, Range, RegisterToolOutput,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PlatformMapper {
    pub arch: HashMap<HostArch, String>,
    pub archs: Vec<HostArch>,
    pub archive_prefix: Option<String>,
    pub checksum_file: Option<String>,
    pub download_file: String,
    pub exes_dirs: Vec<PathBuf>,
    pub exe_path: Option<PathBuf>,
    pub libc: HashMap<HostLibc, String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PluginSchema {
    pub description: Option<String>,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ResolveSchema {
    pub version_pattern: String,
    // Manifest
    pub manifest_url: Option<String>,
    pub manifest_version_key: String,
    // Tags
    pub git_url: Option<String>,
    pub git_tag_pattern: Option<String>,
}

impl Default for ResolveSchema {
    fn default() -> Self {
        ResolveSchema {
            manifest_url: None,
            manifest_version_key: "version".to_string(),
            git_url: None,
            git_tag_pattern: None,
            version_pattern: VERSION_REGEX.into(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Override {
    pub detect: DetectVersionOutput,
    pub resolve: ResolveSchema,
    pub install: DownloadPrebuiltOutput,
    pub locate: LocateExecutablesOutput,

    pub platform: HashMap<HostOS, PlatformMapper>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SchemaV2 {
    pub plugin: PluginSchema,
    pub metadata: RegisterToolOutput,

    #[serde(default)]
    pub detect: DetectVersionOutput,
    #[serde(default)]
    pub source: LoadVersionsOutput,
    #[serde(default)]
    pub resolve: ResolveSchema,
    #[serde(default)]
    pub install: DownloadPrebuiltOutput,
    #[serde(default)]
    pub locate: LocateExecutablesOutput,

    #[serde(default)]
    pub platform: HashMap<HostOS, PlatformMapper>,
    #[serde(default)]
    pub overrides: HashMap<Range, Override>,
}
