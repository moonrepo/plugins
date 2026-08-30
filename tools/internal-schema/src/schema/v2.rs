use super::{PlatformMapper, VERSION_REGEX};
use proto_pdk::{
    DetectVersionOutput, DownloadPrebuiltOutput, HostEnvironment, HostOS, LoadVersionsOutput,
    LocateExecutablesOutput, MatchesVersion, PluginError, Range, RegisterToolOutput, VersionSpec,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PluginSchema {
    pub description: Option<String>,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
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

impl SchemaV2 {
    pub fn get_platform(
        &self,
        env: &HostEnvironment,
        spec: Option<&VersionSpec>,
    ) -> Result<PlatformMapper, PluginError> {
        let mut platform = PlatformMapper::find_match(&self.platform, env)
            .ok_or_else(|| PluginError::UnsupportedOS {
                tool: self.metadata.name.clone(),
                os: env.os.to_rust_os(),
            })?
            .to_owned();

        let Some(spec) = spec else {
            return Ok(platform);
        };

        for (range, or) in &self.overrides {
            if let Some(version) = spec.as_version()
                && range.matches(version)
            {
                let platform_override = PlatformMapper::find_match(&or.platform, env);

                if let Some(platform_override) = platform_override {
                    platform.override_with(platform_override);
                }
            }
        }

        Ok(platform)
    }
}
