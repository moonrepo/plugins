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
    pub index_url: Option<String>,
    pub index_version_key: String,
    // Tags
    pub git_url: Option<String>,
    pub git_tag_pattern: Option<String>,
}

impl Default for ResolveSchema {
    fn default() -> Self {
        ResolveSchema {
            index_url: None,
            index_version_key: "version".to_string(),
            git_url: None,
            git_tag_pattern: None,
            version_pattern: VERSION_REGEX.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Override {
    pub range: Range,

    pub resolve: Option<ResolveSchema>,
    pub install: Option<DownloadPrebuiltOutput>, //
    pub locate: Option<LocateExecutablesOutput>, //

    #[serde(default)]
    pub platform: HashMap<HostOS, PlatformMapper>, //
}

#[derive(Debug, Default, Deserialize)]
pub struct SchemaV2 {
    #[serde(default)]
    pub plugin: PluginSchema, //
    pub metadata: RegisterToolOutput, //

    #[serde(default)]
    pub detect: DetectVersionOutput, //
    #[serde(default)]
    pub source: LoadVersionsOutput, //
    #[serde(default)]
    pub resolve: ResolveSchema, //
    #[serde(default)]
    pub install: DownloadPrebuiltOutput, //
    #[serde(default)]
    pub locate: LocateExecutablesOutput, //

    #[serde(default)]
    pub platform: HashMap<HostOS, PlatformMapper>, //
    #[serde(default)]
    pub overrides: Vec<Override>, //
}

impl SchemaV2 {
    pub fn apply_overrides<T>(
        &self,
        spec: &VersionSpec,
        mut base: T,
        op: impl Fn(&mut T, &Override),
    ) -> T {
        for or in &self.overrides {
            if let Some(version) = spec.as_version()
                && or.range.matches(version)
            {
                op(&mut base, or);
            }
        }

        base
    }

    pub fn get_platform(
        &self,
        env: &HostEnvironment,
        spec: Option<&VersionSpec>,
    ) -> Result<PlatformMapper, PluginError> {
        let (os, base) = PlatformMapper::find_match(&self.platform, env).ok_or_else(|| {
            PluginError::UnsupportedOS {
                tool: self.metadata.name.clone(),
                os: env.os.to_rust_os(),
            }
        })?;
        let mut platform = base.to_owned();

        let Some(spec) = spec else {
            return Ok(platform);
        };

        for or in &self.overrides {
            if let Some(version) = spec.as_version()
                && or.range.matches(version)
            {
                // Prefer the host OS, otherwise the OS the base matched with
                if let Some(platform_override) =
                    or.platform.get(&env.os).or_else(|| or.platform.get(&os))
                {
                    platform.override_with(platform_override);
                }
            }
        }

        Ok(platform)
    }
}
