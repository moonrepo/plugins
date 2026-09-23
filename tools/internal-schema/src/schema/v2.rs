use super::{PlatformMapper, VERSION_REGEX};
use proto_pdk::{
    DetectVersionOutput, DownloadPrebuiltOutput, HostEnvironment, HostOS, LoadVersionsOutput,
    LocateExecutablesOutput, MatchesVersion, PluginError, Range, RegisterToolOutput, SpecError,
    VersionSpec,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PluginSchema {
    pub description: Option<String>,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
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

/// Either `canary`, or a range that matches against versions.
#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub enum OverrideRange {
    Canary,
    Range(Range),
}

impl OverrideRange {
    pub fn matches(&self, spec: &VersionSpec) -> bool {
        match (self, spec) {
            (Self::Canary, VersionSpec::Canary) => true,
            (Self::Range(range), VersionSpec::Version(version)) => range.matches(version),
            _ => false,
        }
    }
}

impl TryFrom<String> for OverrideRange {
    type Error = SpecError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "canary" {
            Ok(Self::Canary)
        } else {
            Range::parse(value).map(Self::Range)
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Override {
    pub range: OverrideRange,

    pub resolve: Option<ResolveSchema>,
    pub install: Option<DownloadPrebuiltOutput>, //
    pub locate: Option<LocateExecutablesOutput>, //

    #[serde(default)]
    pub platform: HashMap<HostOS, PlatformMapper>, //
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaV2 {
    pub format: serde_json::Value,

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

/// Settings from the base schema, or from an override that matches the version.
pub struct Layer<'a> {
    pub install: Option<&'a DownloadPrebuiltOutput>,
    pub locate: Option<&'a LocateExecutablesOutput>,
    pub platform: Option<&'a PlatformMapper>,
}

impl SchemaV2 {
    /// Apply the base settings, then each override that matches the version,
    /// in the order they were declared, so that later layers win.
    pub fn apply_layers<T: Default>(
        &self,
        env: &HostEnvironment,
        spec: &VersionSpec,
        op: impl Fn(&mut T, Layer<'_>),
    ) -> Result<T, PluginError> {
        let (os, platform) = self.find_platform(env)?;
        let mut value = T::default();

        op(
            &mut value,
            Layer {
                install: Some(&self.install),
                locate: Some(&self.locate),
                platform: Some(platform),
            },
        );

        for or in &self.overrides {
            if or.range.matches(spec) {
                op(
                    &mut value,
                    Layer {
                        install: or.install.as_ref(),
                        locate: or.locate.as_ref(),
                        // Prefer the host OS, otherwise the OS the base matched with
                        platform: or.platform.get(&env.os).or_else(|| or.platform.get(&os)),
                    },
                );
            }
        }

        Ok(value)
    }

    pub fn get_platform(
        &self,
        env: &HostEnvironment,
        spec: Option<&VersionSpec>,
    ) -> Result<PlatformMapper, PluginError> {
        let Some(spec) = spec else {
            return Ok(self.find_platform(env)?.1.to_owned());
        };

        self.apply_layers(env, spec, |prev: &mut PlatformMapper, layer| {
            if let Some(next) = layer.platform {
                prev.override_with(next);
            }
        })
    }

    fn find_platform(
        &self,
        env: &HostEnvironment,
    ) -> Result<(HostOS, &PlatformMapper), PluginError> {
        PlatformMapper::find_match(&self.platform, env).ok_or_else(|| PluginError::UnsupportedOS {
            tool: self.metadata.name.clone(),
            os: env.os.to_rust_os(),
        })
    }
}
