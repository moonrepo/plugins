use super::{PlatformMapper, VERSION_REGEX};
use proto_pdk::{
    Clause, DetectVersionOutput, DownloadPrebuiltOutput, HostEnvironment, HostOS,
    LoadVersionsOutput, LocateExecutablesOutput, MatchesRequirement, MatchesVersion, Op,
    PluginError, Range, RegisterToolOutput, SpecError, UnresolvedVersionSpec, VersionSpec,
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResolveSchema {
    pub version_pattern: Option<String>,
    // Manifest
    pub index_url: Option<String>,
    pub index_version_key: Option<String>,
    // Tags
    pub git_url: Option<String>,
    pub git_tag_pattern: Option<String>,
}

impl ResolveSchema {
    pub fn get_version_pattern(&self) -> &str {
        self.version_pattern.as_deref().unwrap_or(VERSION_REGEX)
    }

    pub fn get_git_tag_pattern(&self) -> &str {
        self.git_tag_pattern
            .as_deref()
            .unwrap_or_else(|| self.get_version_pattern())
    }

    pub fn get_index_version_key(&self) -> &str {
        self.index_version_key.as_deref().unwrap_or("version")
    }

    pub fn override_with(&mut self, other: &ResolveSchema) {
        // Versions come from either a Git repository or an index, so replace both
        if other.git_url.is_some() || other.index_url.is_some() {
            self.git_url = other.git_url.clone();
            self.index_url = other.index_url.clone();
        }

        if let Some(value) = &other.git_tag_pattern {
            self.git_tag_pattern = Some(value.to_owned());
        }

        if let Some(value) = &other.index_version_key {
            self.index_version_key = Some(value.to_owned());
        }

        if let Some(value) = &other.version_pattern {
            self.version_pattern = Some(value.to_owned());
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

    /// Match against the version being resolved, where a requirement or
    /// range matches if at least one version satisfies both.
    pub fn matches_unresolved(&self, spec: &UnresolvedVersionSpec) -> bool {
        match (self, spec) {
            (Self::Canary, UnresolvedVersionSpec::Canary) => true,
            (Self::Range(range), UnresolvedVersionSpec::Version(version)) => range.matches(version),
            (Self::Range(range), UnresolvedVersionSpec::Requirement(req)) => range.matches_req(req),
            (Self::Range(range), UnresolvedVersionSpec::Range(other)) => other
                .clauses
                .iter()
                .any(|clause| overlaps_clause(range, clause)),
            _ => false,
        }
    }
}

// Versions are ordered, so a clause overlaps one of the range's clauses
// if it overlaps each of the clause's requirements (Helly's theorem)
fn overlaps_clause(range: &Range, clause: &Clause) -> bool {
    let reqs = match clause {
        Clause::All(reqs) => reqs.to_owned(),
        Clause::Between(lower, upper) => vec![
            lower.to_requirement(Op::GreaterEq),
            upper.to_requirement(Op::LessEq),
        ],
        Clause::Only(req) => vec![req.to_owned()],
    };

    if range.clauses.is_empty() {
        return reqs.iter().all(|req| range.matches_req(req));
    }

    range
        .clauses
        .iter()
        .any(|own| reqs.iter().all(|req| own.matches_req(req)))
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

    // Matched against the version being resolved, instead of installed
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

    /// Apply the base resolve settings, then each override that matches the
    /// version being resolved, in the order they were declared.
    pub fn get_resolve(&self, spec: &UnresolvedVersionSpec) -> ResolveSchema {
        let mut resolve = self.resolve.clone();

        for or in &self.overrides {
            if let Some(next) = &or.resolve
                && or.range.matches_unresolved(spec)
            {
                resolve.override_with(next);
            }
        }

        resolve
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

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(range: &str, spec: &str) -> bool {
        OverrideRange::try_from(range.to_owned())
            .unwrap()
            .matches_unresolved(&UnresolvedVersionSpec::parse(spec).unwrap())
    }

    #[test]
    fn matches_versions_and_requirements() {
        assert!(matches("<1", "0.21.3"));
        assert!(!matches("<1", "1.0.0"));
        assert!(matches("<1", "0.21"));
        assert!(matches(">=1.2.5", "~1.2"));
        assert!(!matches("<1", "^1"));
    }

    #[test]
    fn matches_ranges_that_overlap() {
        assert!(matches("<1", ">=0.5 <2"));
        assert!(matches("<1", "^0.1 || ^3"));
        assert!(matches(">=1.5 <3", ">=1 <2"));
        assert!(matches(">=1.5 <3", "1.2.3 - 1.5.0"));
        assert!(!matches("<1", "1.2.3 - 2.0.0"));
        assert!(!matches("<1", ">=1 <2 || ^3"));
        assert!(!matches(">=1.5 <3", ">=1 <1.5"));
        // Each requirement overlaps a different clause, but none overlap both
        assert!(!matches("<1 || >=3", ">=1 <2"));
    }

    #[test]
    fn matches_canary_only_with_canary() {
        assert!(matches("canary", "canary"));
        assert!(!matches("canary", "1.0.0"));
        assert!(!matches("*", "canary"));
        assert!(!matches("<1", "latest"));
    }
}
