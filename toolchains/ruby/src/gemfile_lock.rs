//! Vendored + hardened `Gemfile.lock` parser.
//!
//! The core state machine is adapted from the MIT/Apache-licensed `rubund`
//! crate (github.com/linyiru/rubyrs). Differences vs upstream:
//!   - Returns owned data and a `Result`, so malformed input is surfaced
//!     (upstream is infallible and silently yields an empty struct).
//!   - Splits the platform suffix out of a spec version, e.g.
//!     `nokogiri (1.16.0-x86_64-linux)` -> ("1.16.0", "x86_64-linux"); upstream
//!     folds the platform into the version string.
//!
//! `Gemfile.lock` is the reliable source of truth for Ruby dependencies (the
//! `Gemfile` itself is executable Ruby code).

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Gem,
    Git,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemSource {
    pub type_: SourceType,
    pub remote: String,
    pub revision: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemSpec {
    pub name: String,
    pub version: String,
    /// Platform suffix for platform-specific gems (e.g. `x86_64-linux`).
    pub platform: Option<String>,
    /// Index into [`Lockfile::sources`] this spec was resolved from.
    pub source_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub requirement: Option<String>,
    /// `!` suffix: pinned to a non-registry (path/git) source.
    pub pinned: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Lockfile {
    pub sources: Vec<GemSource>,
    pub specs: Vec<GemSpec>,
    pub platforms: Vec<String>,
    pub dependencies: Vec<Dependency>,
    /// "name version" -> sha256, from the CHECKSUMS section (Bundler >= 2.5).
    pub checksums: HashMap<String, String>,
    pub ruby_version: Option<String>,
    pub bundled_with: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Input was non-empty but contained no recognizable lockfile structure.
    Unrecognized,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Unrecognized => write!(
                f,
                "could not parse Gemfile.lock: no recognizable sections found"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Gem,
    Git,
    Path,
    Platforms,
    Dependencies,
    Checksums,
    RubyVersion,
    BundledWith,
}

fn flush_source(
    lock: &mut Lockfile,
    section: Section,
    remote: &mut String,
    revision: &mut Option<String>,
    branch: &mut Option<String>,
) {
    let type_ = match section {
        Section::Git => SourceType::Git,
        Section::Path => SourceType::Path,
        _ => SourceType::Gem,
    };

    lock.sources.push(GemSource {
        type_,
        remote: std::mem::take(remote),
        revision: revision.take(),
        branch: branch.take(),
    });
}

pub fn parse(content: &str) -> Result<Lockfile, ParseError> {
    let mut lock = Lockfile::default();
    let mut section = Section::None;

    let mut cur_remote = String::new();
    let mut cur_revision = None;
    let mut cur_branch = None;
    let mut parsing_specs = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let indent = line.len() - trimmed.len();

        // Top-level section headers sit at indent 0.
        if indent == 0 {
            if matches!(section, Section::Gem | Section::Git | Section::Path) {
                flush_source(
                    &mut lock,
                    section,
                    &mut cur_remote,
                    &mut cur_revision,
                    &mut cur_branch,
                );
                parsing_specs = false;
            }

            section = match trimmed {
                "GEM" => Section::Gem,
                "GIT" => Section::Git,
                "PATH" => Section::Path,
                "PLATFORMS" => Section::Platforms,
                "DEPENDENCIES" => Section::Dependencies,
                "CHECKSUMS" => Section::Checksums,
                "RUBY VERSION" => Section::RubyVersion,
                "BUNDLED WITH" => Section::BundledWith,
                _ => Section::None,
            };
            continue;
        }

        match section {
            Section::Gem | Section::Git | Section::Path => {
                if indent == 2 {
                    if let Some(rest) = trimmed.strip_prefix("remote:") {
                        cur_remote = rest.trim().to_string();
                    } else if let Some(rest) = trimmed.strip_prefix("revision:") {
                        cur_revision = Some(rest.trim().to_string());
                    } else if let Some(rest) = trimmed.strip_prefix("branch:") {
                        cur_branch = Some(rest.trim().to_string());
                    } else if trimmed == "specs:" {
                        parsing_specs = true;
                    }
                } else if parsing_specs && indent == 4 {
                    // A resolved spec: e.g. "    rails (7.1.3)".
                    if let Some((name, rest)) = trimmed.split_once(' ') {
                        let raw = rest.trim_matches(|c| c == '(' || c == ')' || c == ' ');
                        let (version, platform) = split_platform(raw);

                        lock.specs.push(GemSpec {
                            name: name.trim().to_string(),
                            version,
                            platform,
                            source_index: lock.sources.len(),
                        });
                    }
                }
                // indent 6 = a spec's transitive deps; moon wants the flat
                // resolved set, so we don't track the resolution graph here.
            }
            Section::Platforms => {
                if indent == 2 {
                    lock.platforms.push(trimmed.trim().to_string());
                }
            }
            Section::Dependencies => {
                if indent == 2 {
                    // e.g. "  rails (~> 7.1)" or "  billing!" (pinned).
                    let pinned = trimmed.ends_with('!');
                    let cleaned = trimmed.trim_end_matches('!');
                    let (name, requirement) = match cleaned.split_once(' ') {
                        Some((n, rest)) => (
                            n.trim().to_string(),
                            Some(
                                rest.trim_matches(|c| c == '(' || c == ')' || c == ' ')
                                    .to_string(),
                            ),
                        ),
                        None => (cleaned.trim().to_string(), None),
                    };

                    lock.dependencies.push(Dependency {
                        name,
                        requirement,
                        pinned,
                    });
                }
            }
            Section::Checksums => {
                if indent == 2 {
                    // e.g. "  rake (10.3.2) sha256=814828...".
                    if let Some((name, rest)) = trimmed.split_once(' ')
                        && let Some((version, sha_part)) = rest.trim().split_once(' ')
                    {
                        let version = version.trim_matches(|c| c == '(' || c == ')' || c == ' ');
                        let sha = sha_part.trim().trim_start_matches("sha256=");
                        lock.checksums
                            .insert(format!("{} {}", name.trim(), version), sha.to_string());
                    }
                }
            }
            Section::RubyVersion => lock.ruby_version = Some(trimmed.trim().to_string()),
            Section::BundledWith => lock.bundled_with = Some(trimmed.trim().to_string()),
            Section::None => {}
        }
    }

    if matches!(section, Section::Gem | Section::Git | Section::Path) {
        flush_source(
            &mut lock,
            section,
            &mut cur_remote,
            &mut cur_revision,
            &mut cur_branch,
        );
    }

    // Hardening: surface unparseable input rather than returning empty silently
    // (the gap in upstream `rubund`'s infallible API).
    if !content.trim().is_empty()
        && lock.sources.is_empty()
        && lock.specs.is_empty()
        && lock.dependencies.is_empty()
    {
        return Err(ParseError::Unrecognized);
    }

    Ok(lock)
}

/// Split a lockfile version token into (version, platform). Gem versions never
/// contain `-`; in a spec line a trailing `-<platform>` (e.g. `x86_64-linux`)
/// denotes a platform-specific gem.
fn split_platform(raw: &str) -> (String, Option<String>) {
    match raw.split_once('-') {
        Some((version, platform)) => (version.to_string(), Some(platform.to_string())),
        None => (raw.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"PATH
  remote: ../libs/billing
  specs:
    billing (0.1.0)
      activesupport (>= 7.0)

GIT
  remote: https://github.com/me/some_fork.git
  revision: abc123def456
  branch: main
  specs:
    some_fork (1.2.0)

GEM
  remote: https://rubygems.org/
  specs:
    activesupport (7.1.3)
      concurrent-ruby (~> 1.0)
    concurrent-ruby (1.3.4)
    nokogiri (1.16.0-x86_64-linux)

PLATFORMS
  arm64-darwin-23
  x86_64-linux

DEPENDENCIES
  activesupport (~> 7.1)
  billing!
  nokogiri

CHECKSUMS
  activesupport (7.1.3) sha256=deadbeef

RUBY VERSION
   ruby 3.3.5p100

BUNDLED WITH
   2.5.6
"#;

    #[test]
    fn parses_all_sections() {
        let lock = parse(SAMPLE).unwrap();

        assert_eq!(lock.sources.len(), 3);
        assert_eq!(lock.platforms, vec!["arm64-darwin-23", "x86_64-linux"]);
        assert_eq!(lock.ruby_version.as_deref(), Some("ruby 3.3.5p100"));
        assert_eq!(lock.bundled_with.as_deref(), Some("2.5.6"));
        assert_eq!(
            lock.checksums.get("activesupport 7.1.3").unwrap(),
            "deadbeef"
        );
    }

    #[test]
    fn maps_path_spec_back_to_path_source() {
        let lock = parse(SAMPLE).unwrap();

        let billing = lock.specs.iter().find(|s| s.name == "billing").unwrap();
        assert_eq!(billing.version, "0.1.0");
        assert_eq!(lock.sources[billing.source_index].type_, SourceType::Path);
        assert_eq!(lock.sources[billing.source_index].remote, "../libs/billing");
    }

    #[test]
    fn captures_git_revision_and_branch() {
        let lock = parse(SAMPLE).unwrap();

        let git = lock
            .sources
            .iter()
            .find(|s| s.type_ == SourceType::Git)
            .unwrap();
        assert_eq!(git.revision.as_deref(), Some("abc123def456"));
        assert_eq!(git.branch.as_deref(), Some("main"));
    }

    #[test]
    fn splits_platform_from_version() {
        let lock = parse(SAMPLE).unwrap();

        let nokogiri = lock.specs.iter().find(|s| s.name == "nokogiri").unwrap();
        assert_eq!(nokogiri.version, "1.16.0");
        assert_eq!(nokogiri.platform.as_deref(), Some("x86_64-linux"));
    }

    #[test]
    fn tracks_pinned_dependencies() {
        let lock = parse(SAMPLE).unwrap();

        let billing = lock
            .dependencies
            .iter()
            .find(|d| d.name == "billing")
            .unwrap();
        assert!(billing.pinned);

        let active = lock
            .dependencies
            .iter()
            .find(|d| d.name == "activesupport")
            .unwrap();
        assert!(!active.pinned);
        assert_eq!(active.requirement.as_deref(), Some("~> 7.1"));
    }

    #[test]
    fn errors_on_unrecognized_input() {
        assert_eq!(
            parse("this is not a lockfile\n").unwrap_err(),
            ParseError::Unrecognized
        );
    }

    #[test]
    fn empty_input_is_ok() {
        assert_eq!(parse("").unwrap(), Lockfile::default());
    }
}
