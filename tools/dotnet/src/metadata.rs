use serde::{Deserialize, Deserializer};

/// Microsoft's release metadata. Every SDK version, its per-platform archive
/// URL and its SHA512 come from here.
///
/// Deliberately not git tags. `dotnet/installer` stopped tagging after v8, and
/// `dotnet/sdk` tags neither cover every published SDK (8.0.125, 8.0.201,
/// 9.0.101 and others have no tag) nor guarantee a downloadable archive (some
/// tags 404). Both existing third-party .NET proto plugins resolve from tags
/// and are broken in exactly these ways.
pub static DEFAULT_METADATA_URL: &str =
    "https://builds.dotnet.microsoft.com/dotnet/release-metadata";

/// The metadata writes `null` rather than omitting empty lists, and
/// `#[serde(default)]` only covers a missing field.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
pub struct ReleasesIndex {
    #[serde(rename = "releases-index")]
    pub channels: Vec<ChannelSummary>,
}

#[derive(Debug, Deserialize)]
pub struct ChannelSummary {
    #[serde(rename = "channel-version")]
    pub channel_version: String,

    #[serde(rename = "latest-sdk")]
    pub latest_sdk: String,

    /// `active`, `maintenance`, `preview` or `eol`.
    #[serde(rename = "support-phase", default)]
    pub support_phase: String,

    /// `lts` or `sts`.
    #[serde(rename = "release-type", default)]
    pub release_type: String,
    // The index also carries an absolute `releases.json` URL. It is ignored on
    // purpose: building the URL from `metadata_url` instead is what lets a
    // mirror or air-gapped host redirect every request.
}

impl ChannelSummary {
    pub fn is_preview(&self) -> bool {
        self.support_phase == "preview"
    }

    /// A channel still receiving releases, which is what `lts`/`sts` should
    /// select. Excludes `eol` and `preview`.
    pub fn is_supported(&self) -> bool {
        matches!(self.support_phase.as_str(), "active" | "maintenance")
    }

    /// `major.minor` as numbers, for ordering channels. Non-numeric parts sort
    /// as 0, which only matters if the metadata ever ships a malformed channel.
    pub fn order_key(&self) -> (u32, u32) {
        let mut parts = self.channel_version.split('.');
        let mut next = || {
            parts
                .next()
                .and_then(|part| part.parse::<u32>().ok())
                .unwrap_or(0)
        };

        (next(), next())
    }
}

impl ReleasesIndex {
    /// The `latest-sdk` an alias points at, taken from the index alone.
    ///
    /// Every alias this plugin publishes is a property of a *channel*, and the
    /// index already carries each channel's newest SDK, so answering one costs
    /// the index request and nothing more. Returns `None` for anything else, so
    /// the caller falls back to the full version listing.
    pub fn latest_sdk_for_alias(&self, alias: &str) -> Option<&str> {
        let pick = |wanted: &str| {
            self.channels
                .iter()
                .filter(|channel| channel.is_supported() && channel.release_type == wanted)
                .max_by_key(|channel| channel.order_key())
        };

        let channel = match alias {
            "lts" => pick("lts")?,
            "sts" => pick("sts")?,
            "preview" => self.channels.iter().find(|channel| channel.is_preview())?,
            // `latest` and `stable` mean the newest generally available SDK,
            // which is the highest channel that is not a preview.
            "latest" | "stable" => self
                .channels
                .iter()
                .filter(|channel| !channel.is_preview())
                .max_by_key(|channel| channel.order_key())?,
            _ => return None,
        };

        Some(&channel.latest_sdk)
    }
}

#[derive(Debug, Deserialize)]
pub struct ChannelReleases {
    #[serde(default, deserialize_with = "null_as_default")]
    pub releases: Vec<Release>,
}

#[derive(Debug, Deserialize)]
pub struct Release {
    /// The feature bands published in this release. A release carries several
    /// (`["8.0.424", "8.0.130"]`), so this — not `sdk` — is the complete list.
    #[serde(default, deserialize_with = "null_as_default")]
    pub sdks: Vec<Sdk>,
}

#[derive(Debug, Deserialize)]
pub struct Sdk {
    #[serde(default, deserialize_with = "null_as_default")]
    pub version: String,

    #[serde(default, deserialize_with = "null_as_default")]
    pub files: Vec<SdkFile>,
}

/// Every field here carries `null_as_default` rather than a bare
/// `#[serde(default)]`: the metadata writes `null` for values it has no entry
/// for, and `default` alone only covers a *missing* key — an explicit
/// `"hash": null` is a hard deserialize error that would fail the whole
/// channel.
#[derive(Debug, Deserialize)]
pub struct SdkFile {
    #[serde(default, deserialize_with = "null_as_default")]
    pub name: String,

    /// Platform identifier, e.g. `linux-musl-x64`.
    #[serde(default, deserialize_with = "null_as_default")]
    pub rid: String,

    #[serde(default, deserialize_with = "null_as_default")]
    pub url: String,

    /// SHA512, as 128 hex characters.
    #[serde(default, deserialize_with = "null_as_default")]
    pub hash: String,
}

impl ChannelReleases {
    pub fn sdks(&self) -> impl Iterator<Item = &Sdk> {
        self.releases.iter().flat_map(|release| &release.sdks)
    }

    pub fn find_sdk(&self, version: &str) -> Option<&Sdk> {
        self.sdks().find(|sdk| sdk.version == version)
    }
}

impl Sdk {
    /// The archive for a platform. Matched on `rid` rather than `name`,
    /// because a release ships several files per platform (archive, installer,
    /// and on macOS a `.pkg`).
    pub fn find_archive(&self, rid: &str, extension: &str) -> Option<&SdkFile> {
        let suffix = format!("-{rid}.{extension}");

        self.files
            .iter()
            .find(|file| file.rid == rid && file.name.ends_with(&suffix))
    }
}

/// The metadata channel a version belongs to: `8.0.424` -> `8.0`. Lets a
/// single-version lookup hit one channel file instead of walking them all.
pub fn channel_of(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;

    if major.is_empty() || minor.is_empty() {
        return None;
    }

    Some(format!("{major}.{minor}"))
}

/// The SDK feature band: `8.0.424` -> `4`. Bands are parallel product lines,
/// not a linear sequence, so "highest version wins" can move a pin sideways
/// onto a band the repository never asked for.
pub fn feature_band(version: &str) -> Option<u32> {
    let patch = version.split('.').nth(2)?;
    let digits = patch
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();

    if digits.len() < 3 {
        return None;
    }

    digits[..digits.len() - 2].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_metadata_url_is_the_current_cdn() {
        // dotnetcli.azureedge.net and dotnetcli.blob.core.windows.net are the
        // legacy hosts; they still resolve but should not be baked in.
        assert!(DEFAULT_METADATA_URL.starts_with("https://builds.dotnet.microsoft.com/"));
    }

    #[test]
    fn derives_the_channel() {
        assert_eq!(channel_of("8.0.424").as_deref(), Some("8.0"));
        assert_eq!(channel_of("10.0.100").as_deref(), Some("10.0"));
        assert_eq!(
            channel_of("11.0.100-preview.7.26381.103").as_deref(),
            Some("11.0")
        );
        assert_eq!(channel_of("8").as_deref(), None);
        assert_eq!(channel_of("").as_deref(), None);
    }

    #[test]
    fn derives_the_feature_band() {
        assert_eq!(feature_band("8.0.424"), Some(4));
        assert_eq!(feature_band("8.0.130"), Some(1));
        assert_eq!(feature_band("10.0.100"), Some(1));
        // Pre-release metadata hangs off the patch, and the band still leads it.
        assert_eq!(feature_band("11.0.100-preview.7.26381.103"), Some(1));
        // Two-digit patches predate feature bands (1.0.4, 2.1.14).
        assert_eq!(feature_band("2.1.14"), None);
        assert_eq!(feature_band("8.0"), None);
    }
}
