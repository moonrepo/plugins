//! `global.json` SDK pin parsing.
//!
//! Used to avoid injecting a `DOTNET_ROOT` that cannot satisfy the SDK a
//! workspace pins: the dotnet host resolves `global.json` from the current
//! directory, so a stale root (e.g. a leftover `~/.dotnet`) makes every task
//! fail with the host's own "compatible SDK was not found" error while
//! graph evaluation, running elsewhere, succeeds.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GlobalJsonFile {
    sdk: Option<GlobalJsonSdk>,
    test: Option<GlobalJsonTest>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GlobalJsonTest {
    runner: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GlobalJsonSdk {
    version: Option<String>,
    roll_forward: Option<String>,
    allow_prerelease: Option<bool>,
}

/// How far the dotnet host may roll forward from the pinned SDK version.
///
/// The `latest*` variants differ from their plain counterparts only in *which*
/// matching SDK gets chosen, not in whether one exists, so both map to the
/// same level here — this type only answers "could any installed SDK satisfy
/// this pin?".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollForward {
    Disable,
    Patch,
    Feature,
    Minor,
    Major,
}

impl RollForward {
    fn parse(value: Option<&str>) -> Self {
        match value.map(str::to_ascii_lowercase).as_deref() {
            Some("disable") => Self::Disable,
            // `patch` is the documented default when rollForward is unset.
            None | Some("patch") | Some("latestpatch") => Self::Patch,
            Some("feature") | Some("latestfeature") => Self::Feature,
            Some("minor") | Some("latestminor") => Self::Minor,
            Some("major") | Some("latestmajor") => Self::Major,
            // Unknown values: assume the most permissive level rather than
            // wrongly reporting a pin as unsatisfiable.
            Some(_) => Self::Major,
        }
    }
}

/// An SDK version pinned or installed, as `major.minor.patch` plus whether it
/// carries a prerelease suffix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SdkVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: bool,
}

impl SdkVersion {
    /// SDK feature band — the hundreds component of the patch (e.g. `201`
    /// is band 2). `patch`-level roll-forward stays within one band.
    fn feature_band(&self) -> u64 {
        self.patch / 100
    }

    fn triple(&self) -> (u64, u64, u64) {
        (self.major, self.minor, self.patch)
    }

    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let (numeric, prerelease) = match value.split_once('-') {
            Some((numeric, _)) => (numeric, true),
            None => (value, false),
        };

        let mut parts = numeric.split('.');
        let major = parts.next()?.parse().ok()?;

        // Missing components default to 0, so a lenient "10" or "10.0" pin
        // still compares sensibly even though global.json requires all three.
        let component = |part: Option<&str>| match part {
            Some(value) => value.parse().ok(),
            None => Some(0),
        };

        Some(Self {
            major,
            minor: component(parts.next())?,
            patch: component(parts.next())?,
            prerelease,
        })
    }
}

/// The SDK pin declared by a `global.json`.
#[derive(Clone, Debug)]
pub struct SdkRequirement {
    /// Version string exactly as written, for error messages and
    /// `rollForward: disable` comparisons.
    pub version: String,
    pub parsed: SdkVersion,
    pub roll_forward: RollForward,
    pub allow_prerelease: bool,
}

/// Parse the `sdk` block of a `global.json`. Returns `None` when the file
/// declares no SDK version — there is nothing to satisfy then.
pub fn parse_sdk_requirement(content: &str) -> Option<SdkRequirement> {
    let file: GlobalJsonFile = serde_json::from_str(content).ok()?;
    let sdk = file.sdk?;
    let version = sdk.version?;
    let parsed = SdkVersion::parse(&version)?;

    Some(SdkRequirement {
        version,
        parsed,
        roll_forward: RollForward::parse(sdk.roll_forward.as_deref()),
        // Documented default is to allow prerelease SDKs.
        allow_prerelease: sdk.allow_prerelease.unwrap_or(true),
    })
}

/// Does this `global.json` select Microsoft.Testing.Platform as the runner
/// for `dotnet test` (`{"test": {"runner": "Microsoft.Testing.Platform"}}`)?
///
/// It changes the `dotnet test` command line, not just the runner: MTP takes
/// the project through `--project` and rejects a positional path, while
/// classic VSTest mode is the exact opposite. Verified against SDK 10.0.201.
pub fn selects_test_platform(content: &str) -> bool {
    serde_json::from_str::<GlobalJsonFile>(content)
        .ok()
        .and_then(|file| file.test?.runner)
        .is_some_and(|runner| runner.eq_ignore_ascii_case("Microsoft.Testing.Platform"))
}

/// Could any of these installed SDK versions satisfy the pin?
///
/// Unparseable installed version strings are ignored; an unparseable pin
/// never reaches here (`parse_sdk_requirement` returns `None`).
pub fn satisfies(installed: &[String], requirement: &SdkRequirement) -> bool {
    installed.iter().any(|version| {
        if requirement.roll_forward == RollForward::Disable {
            return version.trim() == requirement.version.trim();
        }

        let Some(candidate) = SdkVersion::parse(version) else {
            return false;
        };

        if candidate.prerelease && !requirement.allow_prerelease {
            return false;
        }

        let pinned = requirement.parsed;

        match requirement.roll_forward {
            RollForward::Disable => unreachable!("handled above"),
            RollForward::Patch => {
                candidate.major == pinned.major
                    && candidate.minor == pinned.minor
                    && candidate.feature_band() == pinned.feature_band()
                    && candidate.patch >= pinned.patch
            }
            RollForward::Feature => {
                candidate.major == pinned.major
                    && candidate.minor == pinned.minor
                    && candidate.patch >= pinned.patch
            }
            RollForward::Minor => {
                candidate.major == pinned.major && candidate.triple() >= pinned.triple()
            }
            RollForward::Major => candidate.triple() >= pinned.triple(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn versions(list: &[&str]) -> Vec<String> {
        list.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_versions_with_and_without_prerelease() {
        let version = SdkVersion::parse("10.0.201").unwrap();
        assert_eq!(version.triple(), (10, 0, 201));
        assert!(!version.prerelease);
        assert_eq!(version.feature_band(), 2);

        let prerelease = SdkVersion::parse("10.0.100-rc.1.25451.107").unwrap();
        assert_eq!(prerelease.triple(), (10, 0, 100));
        assert!(prerelease.prerelease);

        // Lenient about missing components.
        assert_eq!(SdkVersion::parse("8").unwrap().triple(), (8, 0, 0));
        assert_eq!(SdkVersion::parse("8.0").unwrap().triple(), (8, 0, 0));
        assert!(SdkVersion::parse("").is_none());
        assert!(SdkVersion::parse("latest").is_none());
    }

    #[test]
    fn parses_a_global_json_carrying_unrelated_keys() {
        // Shape taken from a production repository: `msbuild-sdks` and `test`
        // sit alongside `sdk`, and neither must interfere with the pin.
        let requirement = parse_sdk_requirement(
            r#"{
              "sdk": {
                "version": "10.0.301",
                "rollForward": "latestMajor",
                "allowPrerelease": true
              },
              "msbuild-sdks": { "Aspire.AppHost.Sdk": "13.4.6" },
              "test": { "runner": "Microsoft.Testing.Platform" }
            }"#,
        )
        .unwrap();

        assert_eq!(requirement.version, "10.0.301");
        assert_eq!(requirement.roll_forward, RollForward::Major);
        assert!(requirement.allow_prerelease);
    }

    #[test]
    fn detects_the_microsoft_testing_platform_runner() {
        // Real shape from a production repo.
        assert!(selects_test_platform(
            r#"{
              "sdk": { "version": "10.0.301", "rollForward": "latestMajor" },
              "test": { "runner": "Microsoft.Testing.Platform" }
            }"#
        ));
        assert!(selects_test_platform(
            r#"{"test":{"runner":"microsoft.testing.platform"}}"#
        ));

        assert!(!selects_test_platform(r#"{"test":{"runner":"VSTest"}}"#));
        assert!(!selects_test_platform(r#"{"test":{}}"#));
        assert!(!selects_test_platform(r#"{"sdk":{"version":"10.0.301"}}"#));
        assert!(!selects_test_platform("{}"));
        assert!(!selects_test_platform("not json"));
    }

    #[test]
    fn no_requirement_without_a_version() {
        assert!(parse_sdk_requirement("{}").is_none());
        assert!(parse_sdk_requirement(r#"{"sdk":{}}"#).is_none());
        assert!(parse_sdk_requirement(r#"{"sdk":{"rollForward":"major"}}"#).is_none());
        assert!(parse_sdk_requirement("not json").is_none());
    }

    #[test]
    fn latest_major_accepts_any_newer_sdk() {
        let requirement =
            parse_sdk_requirement(r#"{"sdk":{"version":"10.0.301","rollForward":"latestMajor"}}"#)
                .unwrap();

        // The case this guard exists for: a leftover `~/.dotnet` holding only
        // SDK 8 cannot serve a 10.x pin, so it must not be preferred over the
        // `dotnet` on PATH.
        assert!(!satisfies(&versions(&["8.0.423"]), &requirement));
        assert!(satisfies(&versions(&["8.0.423", "10.0.301"]), &requirement));
        assert!(satisfies(&versions(&["11.0.100"]), &requirement));
        assert!(!satisfies(&versions(&["10.0.201"]), &requirement));
    }

    #[test]
    fn default_roll_forward_stays_within_the_feature_band() {
        let requirement = parse_sdk_requirement(r#"{"sdk":{"version":"10.0.201"}}"#).unwrap();

        assert_eq!(requirement.roll_forward, RollForward::Patch);
        assert!(satisfies(&versions(&["10.0.201"]), &requirement));
        assert!(satisfies(&versions(&["10.0.204"]), &requirement));
        // Higher feature band (3xx) and a different minor are out of reach.
        assert!(!satisfies(&versions(&["10.0.301"]), &requirement));
        assert!(!satisfies(&versions(&["10.1.201"]), &requirement));
        assert!(!satisfies(&versions(&["10.0.104"]), &requirement));
    }

    #[test]
    fn feature_and_minor_levels_widen_progressively() {
        let feature =
            parse_sdk_requirement(r#"{"sdk":{"version":"10.0.201","rollForward":"feature"}}"#)
                .unwrap();
        assert!(satisfies(&versions(&["10.0.301"]), &feature));
        assert!(!satisfies(&versions(&["10.1.100"]), &feature));

        let minor =
            parse_sdk_requirement(r#"{"sdk":{"version":"10.0.201","rollForward":"latestMinor"}}"#)
                .unwrap();
        assert!(satisfies(&versions(&["10.1.100"]), &minor));
        assert!(!satisfies(&versions(&["11.0.100"]), &minor));
    }

    #[test]
    fn disable_requires_an_exact_match() {
        let requirement =
            parse_sdk_requirement(r#"{"sdk":{"version":"10.0.201","rollForward":"disable"}}"#)
                .unwrap();

        assert!(satisfies(&versions(&["10.0.201"]), &requirement));
        assert!(!satisfies(&versions(&["10.0.202"]), &requirement));
    }

    #[test]
    fn prerelease_sdks_only_count_when_allowed() {
        let allowed = parse_sdk_requirement(
            r#"{"sdk":{"version":"10.0.100","rollForward":"latestMajor","allowPrerelease":true}}"#,
        )
        .unwrap();
        assert!(satisfies(&versions(&["10.0.100-rc.1.25451.107"]), &allowed));

        let denied = parse_sdk_requirement(
            r#"{"sdk":{"version":"10.0.100","rollForward":"latestMajor","allowPrerelease":false}}"#,
        )
        .unwrap();
        assert!(!satisfies(&versions(&["10.0.100-rc.1.25451.107"]), &denied));
    }

    #[test]
    fn unknown_roll_forward_values_stay_permissive() {
        let requirement = parse_sdk_requirement(
            r#"{"sdk":{"version":"8.0.100","rollForward":"someFutureMode"}}"#,
        )
        .unwrap();

        assert!(satisfies(&versions(&["10.0.201"]), &requirement));
    }
}
