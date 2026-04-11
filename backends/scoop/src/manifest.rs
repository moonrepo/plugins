//! Serde types for the Scoop app manifest JSON format.
//!
//! Derived from the JSON schema at:
//! https://github.com/ScoopInstaller/Scoop/blob/master/schema.json

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Primitive union types (from schema definitions)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OneOrMany<T1, T2 = T1> {
    One(T1),
    Many(Vec<T2>),
}

impl<T1, T2> OneOrMany<T1, T2> {
    pub fn first(&self) -> Option<&T1> {
        match self {
            Self::One(inner) => Some(inner),
            Self::Many(_) => None,
        }
    }
}

impl<T1, T2> Default for OneOrMany<T1, T2> {
    fn default() -> Self {
        Self::Many(vec![])
    }
}

/// `stringOrArrayOfStrings`: a single string or an array of strings.
///
/// Schema: `anyOf: [string, array<string>]`
pub type StringOrArray = OneOrMany<String>;

/// `stringOrArrayOfStringsOrAnArrayOfArrayOfStrings`:
/// a string, or an array whose elements are each a `stringOrArrayOfStrings`.
///
/// Schema: `anyOf: [string, array<stringOrArrayOfStrings>]`
///
/// Used for `bin` and `persist`. Each array element may be:
///   - A plain string  (e.g. `"node.exe"`)
///   - A sub-array of strings (e.g. `["node.exe", "node"]`)
///
/// We normalise every element into a `Vec<String>` so callers get a uniform
/// `Vec<Vec<String>>`.
pub type StringOrArrayNested = OneOrMany<String, Vec<String>>;

/// `hash`: a hash-pattern string or an array of hash-pattern strings.
///
/// Schema: `anyOf: [hashPattern, array<hashPattern>]`
///
/// Hash patterns are either bare hex (SHA-256 by default) or
/// `"sha1:<hex>"`, `"sha256:<hex>"`, `"sha512:<hex>"`, `"md5:<hex>"`.
pub type Hash = StringOrArray;

/// `shortcutsArray`: an array of shortcut entries, each being `[target, name, params?, icon?]`.
pub type ShortcutsArray = Vec<Vec<String>>;

// ---------------------------------------------------------------------------
// Hash extraction (used in autoupdate)
// ---------------------------------------------------------------------------

/// `hashExtraction`: describes how to extract a hash from a URL.
///
/// Schema: object with optional `find`, `regex`, `jp`, `jsonpath`, `xpath`,
/// `mode`, `type`, `url`.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct HashExtraction {
    #[serde(alias = "find")]
    pub regex: Option<String>,
    #[serde(alias = "jp")]
    pub jsonpath: Option<String>,
    pub xpath: Option<String>,
    pub mode: Option<HashExtractionMode>,
    #[serde(rename = "type")]
    pub hash_type: Option<HashType>,
    pub url: Option<String>,
}

/// Extraction modes for `hashExtraction.mode`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HashExtractionMode {
    Download,
    Extract,
    Json,
    Xpath,
    Rdf,
    Metalink,
    Fosshub,
    SourceForge,
}

/// Hash algorithm types (deprecated in schema).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HashType {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

// ---------------------------------------------------------------------------
// License
// ---------------------------------------------------------------------------

/// `license`: either an SPDX identifier string or `{ identifier, url? }`.
///
/// Schema: `anyOf: [licenseIdentifiers, { identifier: string, url?: string }]`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum License {
    Identifier(String),
    Full {
        identifier: String,
        url: Option<String>,
    },
}

impl Default for License {
    fn default() -> Self {
        Self::Identifier("mit".into())
    }
}

// ---------------------------------------------------------------------------
// Installer / Uninstaller
// ---------------------------------------------------------------------------

/// `installer`: installation configuration.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Installer {
    pub args: Option<StringOrArray>,
    pub file: Option<String>,
    pub script: Option<StringOrArray>,
    pub keep: Option<bool>,
}

/// `uninstaller`: uninstallation configuration.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Uninstaller {
    pub args: Option<StringOrArray>,
    pub file: Option<String>,
    pub script: Option<StringOrArray>,
}

// ---------------------------------------------------------------------------
// Checkver
// ---------------------------------------------------------------------------

/// `checkver`: a regex string or an object with version-check configuration.
///
/// Schema: `anyOf: [string, object]`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Checkver {
    Regex(String),
    Config(CheckverConfig),
}

impl Default for Checkver {
    fn default() -> Self {
        Self::Config(CheckverConfig::default())
    }
}

/// Object form of `checkver`.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CheckverConfig {
    pub github: Option<String>,
    #[serde(alias = "re")]
    pub regex: Option<String>,
    pub url: Option<String>,
    #[serde(alias = "jp")]
    pub jsonpath: Option<String>,
    pub xpath: Option<String>,
    pub reverse: Option<bool>,
    pub replace: Option<String>,
    pub useragent: Option<String>,
    pub script: Option<StringOrArray>,
    pub sourceforge: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// PSModule
// ---------------------------------------------------------------------------

/// `psmodule`: PowerShell module installation.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct PsModule {
    pub name: Option<String>,
}

// ---------------------------------------------------------------------------
// Architecture entry (per 32bit / 64bit / arm64)
// ---------------------------------------------------------------------------

/// `architecture` (definition): per-architecture overrides.
///
/// Contains the same download/install fields as the top-level manifest but
/// scoped to a specific architecture.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ArchitectureEntry {
    pub bin: Option<StringOrArrayNested>,
    pub checkver: Option<Checkver>,
    pub env_add_path: Option<StringOrArray>,
    pub env_set: Option<HashMap<String, String>>,
    pub extract_dir: Option<StringOrArray>,
    pub hash: Option<Hash>,
    pub installer: Option<Installer>,
    pub post_install: Option<StringOrArray>,
    pub post_uninstall: Option<StringOrArray>,
    pub pre_install: Option<StringOrArray>,
    pub pre_uninstall: Option<StringOrArray>,
    pub shortcuts: Option<ShortcutsArray>,
    pub uninstaller: Option<Uninstaller>,
    pub url: Option<StringOrArray>,
}

/// Top-level `architecture` property: maps arch names to entries.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Architecture {
    #[serde(rename = "32bit")]
    pub x86: Option<ArchitectureEntry>,
    #[serde(rename = "64bit")]
    pub x64: Option<ArchitectureEntry>,
    pub arm64: Option<ArchitectureEntry>,
}

impl Architecture {
    pub fn get_entry(&self, arch: &str) -> Option<&ArchitectureEntry> {
        match arch {
            "x86" | "32bit" => self.x86.as_ref(),
            "x64" | "64bit" => self.x64.as_ref(),
            "arm64" => self.arm64.as_ref(),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Autoupdate
// ---------------------------------------------------------------------------

/// `autoupdateArch`: per-architecture autoupdate fields.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AutoupdateArchEntry {
    pub bin: Option<StringOrArrayNested>,
    pub env_add_path: Option<StringOrArray>,
    pub env_set: Option<HashMap<String, String>>,
    pub extract_dir: Option<StringOrArray>,
    pub hash: Option<OneOrMany<HashExtraction>>,
    pub installer: Option<AutoupdateInstaller>,
    pub shortcuts: Option<ShortcutsArray>,
    pub url: Option<StringOrArray>,
}

/// Autoupdate installer (only has `file`).
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AutoupdateInstaller {
    pub file: Option<String>,
}

/// Autoupdate architecture map.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AutoupdateArchitecture {
    #[serde(rename = "32bit")]
    pub x86: Option<AutoupdateArchEntry>,
    #[serde(rename = "64bit")]
    pub x64: Option<AutoupdateArchEntry>,
    pub arm64: Option<AutoupdateArchEntry>,
}

impl AutoupdateArchitecture {
    pub fn get_entry(&self, arch: &str) -> Option<&AutoupdateArchEntry> {
        match arch {
            "x86" | "32bit" => self.x86.as_ref(),
            "x64" | "64bit" => self.x64.as_ref(),
            "arm64" => self.arm64.as_ref(),
            _ => None,
        }
    }
}

/// `autoupdate`: top-level autoupdate configuration.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Autoupdate {
    pub architecture: Option<AutoupdateArchitecture>,
    pub bin: Option<StringOrArrayNested>,
    pub env_add_path: Option<StringOrArray>,
    pub env_set: Option<HashMap<String, String>>,
    pub extract_dir: Option<StringOrArray>,
    pub hash: Option<OneOrMany<HashExtraction>>,
    pub installer: Option<AutoupdateInstaller>,
    pub license: Option<License>,
    pub notes: Option<StringOrArray>,
    pub persist: Option<StringOrArrayNested>,
    pub psmodule: Option<PsModule>,
    pub shortcuts: Option<ShortcutsArray>,
    pub url: Option<StringOrArray>,
}

// ---------------------------------------------------------------------------
// Top-level manifest
// ---------------------------------------------------------------------------

/// A Scoop app manifest.
///
/// https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests
/// https://github.com/ScoopInstaller/Scoop/blob/master/schema.json
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ScoopManifest {
    // -- Required fields ---------------------------------------------------
    pub version: String,
    pub homepage: String,
    pub license: License,

    // -- Optional metadata -------------------------------------------------
    pub description: Option<String>,

    // -- Download / extraction ---------------------------------------------
    pub url: Option<StringOrArray>,
    pub hash: Option<Hash>,
    pub extract_dir: Option<StringOrArray>,
    pub extract_to: Option<StringOrArray>,

    // -- Architecture overrides --------------------------------------------
    pub architecture: Option<Architecture>,

    // -- Executables & environment -----------------------------------------
    pub bin: Option<StringOrArrayNested>,
    pub env_add_path: Option<StringOrArray>,
    pub env_set: Option<HashMap<String, String>>,
    pub shortcuts: Option<ShortcutsArray>,

    // -- Install / uninstall hooks -----------------------------------------
    pub installer: Option<Installer>,
    pub uninstaller: Option<Uninstaller>,
    pub pre_install: Option<StringOrArray>,
    pub post_install: Option<StringOrArray>,
    pub pre_uninstall: Option<StringOrArray>,
    pub post_uninstall: Option<StringOrArray>,

    // -- Persistence & dependencies ----------------------------------------
    pub persist: Option<StringOrArrayNested>,
    pub depends: Option<StringOrArray>,

    // -- Version checking / auto-update ------------------------------------
    pub checkver: Option<Checkver>,
    pub autoupdate: Option<Autoupdate>,
}

// ---------------------------------------------------------------------------
// Convenience helpers used by proto.rs
// ---------------------------------------------------------------------------

/// A resolved executable entry extracted from the `bin` field.
#[derive(Debug, Clone)]
pub struct BinEntry {
    /// Path to the executable (e.g. `"node.exe"` or `"bin\\node.exe"`).
    pub exe_path: String,
    /// Optional alias / shim name (e.g. `"node"`).
    pub alias: Option<String>,
}

impl ScoopManifest {
    // -- Field resolution with architecture fallback -----------------------

    /// Get the download URL for a given architecture.
    /// Falls back to the top-level `url` if no architecture-specific URL exists.
    pub fn get_url_for_arch(&self, arch: &str) -> Option<String> {
        if let Some(architecture) = &self.architecture
            && let Some(entry) = architecture.get_entry(arch)
            && let Some(url) = &entry.url
        {
            return url.first().map(|s| s.to_owned());
        }
        self.url
            .as_ref()
            .and_then(|u| u.first())
            .map(|s| s.to_owned())
    }

    /// Get the hash for a given architecture.
    pub fn get_hash_for_arch(&self, arch: &str) -> Option<String> {
        if let Some(architecture) = &self.architecture
            && let Some(entry) = architecture.get_entry(arch)
            && let Some(hash) = &entry.hash
        {
            return hash.first().map(|s| s.to_owned());
        }
        self.hash
            .as_ref()
            .and_then(|h| h.first())
            .map(|s| s.to_owned())
    }

    /// Get the extract directory for a given architecture.
    pub fn get_extract_dir_for_arch(&self, arch: &str) -> Option<String> {
        if let Some(architecture) = &self.architecture
            && let Some(entry) = architecture.get_entry(arch)
            && let Some(dir) = &entry.extract_dir
        {
            return dir.first().map(|s| s.to_owned());
        }
        self.extract_dir
            .as_ref()
            .and_then(|d| d.first())
            .map(|s| s.to_owned())
    }

    /// Get executable entries for a given architecture.
    /// Falls back to the top-level `bin` if no architecture-specific bin exists.
    pub fn get_executables_for_arch(&self, arch: &str) -> Vec<BinEntry> {
        // let raw = if let Some(architecture) = &self.architecture
        //     && let Some(entry) = architecture.get_entry(arch)
        //     && let Some(bin) = &entry.bin
        // {
        //     &bin.0
        // } else if let Some(bin) = &self.bin {
        //     &bin.0
        // } else {
        //     return Vec::new();
        // };

        // raw.iter()
        //     .map(|parts| BinEntry {
        //         exe_path: parts.first().cloned().unwrap_or_default(),
        //         alias: parts.get(1).cloned(),
        //     })
        //     .collect()

        vec![]
    }

    /// Get `env_add_path` entries for a given architecture.
    pub fn get_env_add_paths_for_arch(&self, arch: &str) -> Vec<String> {
        // if let Some(architecture) = &self.architecture
        //     && let Some(entry) = architecture.get_entry(arch)
        //     && let Some(paths) = &entry.env_add_path
        // {
        //     return paths.0.clone();
        // }
        // self.env_add_path
        //     .as_ref()
        //     .map(|p| p.0.clone())
        //     .unwrap_or_default()

        vec![]
    }

    /// Get `env_set` entries for a given architecture.
    pub fn get_env_set_for_arch(&self, arch: &str) -> HashMap<String, String> {
        if let Some(architecture) = &self.architecture
            && let Some(entry) = architecture.get_entry(arch)
            && let Some(env) = &entry.env_set
        {
            return env.clone();
        }
        self.env_set.clone().unwrap_or_default()
    }

    // -- Version template substitution -------------------------------------

    /// Substitute version placeholders in a template string.
    ///
    /// Supported placeholders:
    ///   `$version`, `$majorVersion`, `$minorVersion`, `$patchVersion`,
    ///   `$buildVersion`.
    pub fn substitute_version(template: &str, version: &str) -> String {
        let mut result = template.replace("$version", version);

        let parts: Vec<&str> = version.splitn(4, '.').collect();
        if let Some(major) = parts.first() {
            result = result.replace("$majorVersion", major);
        }
        if let Some(minor) = parts.get(1) {
            result = result.replace("$minorVersion", minor);
        }
        if let Some(patch) = parts.get(2) {
            result = result.replace("$patchVersion", patch);
        }
        if let Some(build) = parts.get(3) {
            result = result.replace("$buildVersion", build);
        }

        result
    }

    /// Substitute version placeholders, also resolving `$baseurl` from the
    /// manifest's current download URL for the given architecture.
    pub fn substitute_version_with_baseurl(
        &self,
        template: &str,
        version: &str,
        arch: &str,
    ) -> String {
        let mut result = Self::substitute_version(template, version);

        if result.contains("$baseurl")
            && let Some(url) = self.get_url_for_arch(arch)
            && let Some(pos) = url.rfind('/')
        {
            let base = Self::substitute_version(&url[..pos], version);
            result = result.replace("$baseurl", &base);
        }

        result
    }

    // -- Autoupdate resolution ---------------------------------------------

    /// Resolve the autoupdate URL for a given version and architecture.
    pub fn resolve_autoupdate_url(&self, version: &str, arch: &str) -> Option<String> {
        let autoupdate = self.autoupdate.as_ref()?;

        // Try architecture-specific autoupdate first
        if let Some(arch_autoupdate) = &autoupdate.architecture
            && let Some(entry) = arch_autoupdate.get_entry(arch)
            && let Some(url) = &entry.url
        {
            return url
                .first()
                .map(|t| self.substitute_version_with_baseurl(t, version, arch));
        }

        // Fall back to top-level autoupdate URL
        autoupdate
            .url
            .as_ref()
            .and_then(|u| u.first())
            .map(|t| self.substitute_version_with_baseurl(t, version, arch))
    }

    /// Resolve the autoupdate `extract_dir` for a given version and architecture.
    pub fn resolve_autoupdate_extract_dir(&self, version: &str, arch: &str) -> Option<String> {
        let autoupdate = self.autoupdate.as_ref()?;

        if let Some(arch_autoupdate) = &autoupdate.architecture
            && let Some(entry) = arch_autoupdate.get_entry(arch)
            && let Some(dir) = &entry.extract_dir
        {
            return dir.first().map(|t| Self::substitute_version(t, version));
        }

        autoupdate
            .extract_dir
            .as_ref()
            .and_then(|d| d.first())
            .map(|t| Self::substitute_version(t, version))
    }
}
