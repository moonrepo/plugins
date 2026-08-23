use crate::metadata::feature_band;
use proto_pdk_api::{AnyResult, UnresolvedVersionSpec};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct GlobalJson {
    #[serde(default)]
    pub sdk: Option<SdkSection>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SdkSection {
    #[serde(default)]
    pub version: Option<String>,

    #[serde(rename = "rollForward", default)]
    pub roll_forward: Option<String>,
}

/// Translate a `global.json` SDK pin into a version spec.
///
/// Every mode maps exactly. A feature band — the hundreds component of the
/// patch — needs two bounds rather than one, which a compound requirement
/// expresses fine (`>=8.0.404 && <8.0.500`). The important property across all
/// of them is that `rollForward` only ever rolls *forward*: no mode may resolve
/// below the pinned version, so the pinned patch has to survive into the spec.
///
/// `latestPatch` is the SDK's own default when `rollForward` is absent but a
/// version is present, and it is band-locked — which is why the default is the
/// compound range and not `~{version}`.
///
/// `allowPrerelease` is the one thing not expressed. A spec cannot say "include
/// pre-releases", but it does not need to: ranges exclude them by semver
/// convention, and an explicitly pinned pre-release passes through as itself.
/// That matches the SDK's own behaviour closely enough.
pub fn parse(content: &str) -> AnyResult<Option<UnresolvedVersionSpec>> {
    let global: GlobalJson = serde_json::from_str(content)?;

    let Some(sdk) = global.sdk else {
        return Ok(None);
    };

    // `rollForward` on its own selects nothing without a version to roll from.
    let Some(version) = sdk.version else {
        return Ok(None);
    };

    let version = version.trim();

    if version.is_empty() {
        return Ok(None);
    }

    let spec = match sdk.roll_forward.as_deref() {
        // Exactly this SDK, nothing else.
        Some("disable") => version.to_owned(),

        // Roll across feature bands but stay within the major.minor. `~` keeps
        // the pinned patch as the lower bound, so this cannot select a band
        // below the pinned one.
        Some("feature" | "latestFeature") => format!("~{version}"),

        // Roll across minors but stay within the major, again never below the
        // pin.
        Some("minor" | "latestMinor") => format!("^{version}"),

        // Anything at or above the pin.
        Some("major" | "latestMajor") => format!(">={version}"),

        // `patch`, `latestPatch`, an unknown value, or absent: the SDK's own
        // default, which stays inside the pinned feature band. An unrecognised
        // mode is treated as the default rather than rejected, since new modes
        // are added over time.
        _ => match band_upper_bound(version) {
            Some(upper) => format!(">={version} && <{upper}"),
            // Versions predating feature bands (a two-digit patch, as in
            // `2.1.14`) have no band to stay inside, so the major.minor is the
            // tightest honest bound.
            None => format!("~{version}"),
        },
    };

    Ok(Some(UnresolvedVersionSpec::parse(spec)?))
}

/// Exclusive upper bound of a version's own feature band: `8.0.404` ->
/// `8.0.500`. `None` when the version carries no band at all.
fn band_upper_bound(version: &str) -> Option<String> {
    let band = feature_band(version)?;
    let mut parts = version.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;

    Some(format!("{major}.{minor}.{}", (band + 1) * 100))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(content: &str) -> Option<String> {
        parse(content).unwrap().map(|spec| spec.to_string())
    }

    #[test]
    fn no_sdk_section_is_no_pin() {
        assert_eq!(spec("{}"), None);
        assert_eq!(spec(r#"{"msbuild-sdks": {"a": "1.0.0"}}"#), None);
    }

    #[test]
    fn roll_forward_without_a_version_is_no_pin() {
        assert_eq!(spec(r#"{"sdk": {"rollForward": "latestMajor"}}"#), None);
    }

    #[test]
    fn defaults_to_the_pinned_feature_band() {
        // The SDK's default is `latestPatch`, which is band-locked: 8.0.500 is
        // a different band and a `~8.0.404` mapping would wrongly accept it.
        assert_eq!(
            spec(r#"{"sdk": {"version": "8.0.404"}}"#).as_deref(),
            Some(">=8.0.404 && <8.0.500")
        );
        assert_eq!(
            spec(r#"{"sdk": {"version": "9.0.101"}}"#).as_deref(),
            Some(">=9.0.101 && <9.0.200")
        );
    }

    #[test]
    fn versions_predating_feature_bands_fall_back_to_the_major_minor() {
        // Two-digit patches (1.0.4, 2.1.14) have no band to stay inside.
        assert_eq!(
            spec(r#"{"sdk": {"version": "2.1.14"}}"#).as_deref(),
            Some("~2.1.14")
        );
    }

    #[test]
    fn disable_pins_exactly() {
        assert_eq!(
            spec(r#"{"sdk": {"version": "8.0.404", "rollForward": "disable"}}"#).as_deref(),
            Some("8.0.404")
        );
    }

    #[test]
    fn feature_rolls_across_bands_but_never_below_the_pin() {
        // `~8.0` would have matched 8.0.100, i.e. a *lower* band than the pin —
        // something `rollForward` never does.
        assert_eq!(
            spec(r#"{"sdk": {"version": "8.0.404", "rollForward": "latestFeature"}}"#).as_deref(),
            Some("~8.0.404")
        );
    }

    #[test]
    fn minor_and_major_widen_without_going_backwards() {
        assert_eq!(
            spec(r#"{"sdk": {"version": "8.0.404", "rollForward": "latestMinor"}}"#).as_deref(),
            Some("^8.0.404")
        );
        assert_eq!(
            spec(r#"{"sdk": {"version": "8.0.404", "rollForward": "latestMajor"}}"#).as_deref(),
            Some(">=8.0.404")
        );
    }

    #[test]
    fn every_mode_keeps_the_pin_as_its_lower_bound() {
        // The invariant behind all of the above: `rollForward` rolls forward
        // only, so no mode may resolve an SDK older than the pinned one.
        for mode in [
            "disable",
            "patch",
            "latestPatch",
            "feature",
            "latestFeature",
            "minor",
            "latestMinor",
            "major",
            "latestMajor",
        ] {
            let content =
                format!(r#"{{"sdk": {{"version": "8.0.404", "rollForward": "{mode}"}}}}"#);
            let rendered = spec(&content).expect("no spec produced");

            assert!(
                rendered.contains("8.0.404"),
                "{mode} dropped the pinned patch: {rendered}"
            );
        }
    }

    #[test]
    fn unknown_roll_forward_falls_back_to_the_default() {
        assert_eq!(
            spec(r#"{"sdk": {"version": "8.0.404", "rollForward": "somethingNew"}}"#).as_deref(),
            Some(">=8.0.404 && <8.0.500")
        );
    }

    #[test]
    fn prerelease_pins_pass_through() {
        assert_eq!(
            spec(r#"{"sdk": {"version": "11.0.100-preview.7.26381.103", "rollForward": "disable", "allowPrerelease": true}}"#)
                .as_deref(),
            Some("11.0.100-preview.7.26381.103")
        );
    }
}
