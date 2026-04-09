use serde::Deserialize;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use std::collections::HashMap;
use std::fmt;

/// A value that can be either a single string or an array of strings.
#[derive(Debug, Clone, Default)]
pub struct StringOrArray(pub Vec<String>);

impl StringOrArray {
    pub fn first(&self) -> Option<&str> {
        self.0.first().map(|s| s.as_str())
    }
}

impl<'de> Deserialize<'de> for StringOrArray {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StringOrArrayVisitor;

        impl<'de> Visitor<'de> for StringOrArrayVisitor {
            type Value = StringOrArray;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or array of strings")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(StringOrArray(vec![v.to_owned()]))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = seq.next_element::<String>()? {
                    values.push(v);
                }
                Ok(StringOrArray(values))
            }
        }

        deserializer.deserialize_any(StringOrArrayVisitor)
    }
}

/// An executable entry from the `bin` field.
#[derive(Debug, Clone)]
pub struct BinEntry {
    pub exe_path: String,
    pub alias: Option<String>,
}

/// The `bin` field can be a string, array of strings, or array of [exe, alias, ...] arrays.
#[derive(Debug, Clone, Default)]
pub struct ScoopBin(pub Vec<BinEntry>);

impl<'de> Deserialize<'de> for ScoopBin {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ScoopBinVisitor;

        impl<'de> Visitor<'de> for ScoopBinVisitor {
            type Value = ScoopBin;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string, array of strings, or array of [exe, alias] arrays")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(ScoopBin(vec![BinEntry {
                    exe_path: v.to_owned(),
                    alias: None,
                }]))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();

                while let Some(value) = seq.next_element::<serde_json::Value>()? {
                    match value {
                        serde_json::Value::String(s) => {
                            entries.push(BinEntry {
                                exe_path: s,
                                alias: None,
                            });
                        }
                        serde_json::Value::Array(arr) => {
                            let exe_path = arr
                                .first()
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                    de::Error::custom("bin array entry must have an exe path")
                                })?
                                .to_owned();
                            let alias = arr.get(1).and_then(|v| v.as_str()).map(|s| s.to_owned());
                            entries.push(BinEntry { exe_path, alias });
                        }
                        _ => {
                            return Err(de::Error::custom("unexpected bin entry type"));
                        }
                    }
                }

                Ok(ScoopBin(entries))
            }
        }

        deserializer.deserialize_any(ScoopBinVisitor)
    }
}

/// Per-architecture download configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoopArchEntry {
    pub url: Option<StringOrArray>,
    pub hash: Option<StringOrArray>,
    pub extract_dir: Option<StringOrArray>,
    pub bin: Option<ScoopBin>,
    pub env_add_path: Option<StringOrArray>,
}

/// Architecture-specific overrides.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoopArchitecture {
    #[serde(rename = "32bit")]
    pub x32: Option<ScoopArchEntry>,
    #[serde(rename = "64bit")]
    pub x64: Option<ScoopArchEntry>,
    pub arm64: Option<ScoopArchEntry>,
}

impl ScoopArchitecture {
    pub fn get_entry(&self, arch: &str) -> Option<&ScoopArchEntry> {
        match arch {
            "32bit" => self.x32.as_ref(),
            "64bit" => self.x64.as_ref(),
            "arm64" => self.arm64.as_ref(),
            _ => None,
        }
    }
}

/// Autoupdate hash configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ScoopAutoupdateHash {
    Url(String),
    Config {
        url: Option<String>,
        regex: Option<String>,
        jsonpath: Option<String>,
    },
}

/// Autoupdate architecture entry.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoopAutoupdateArchEntry {
    pub url: Option<StringOrArray>,
    pub hash: Option<ScoopAutoupdateHash>,
    pub extract_dir: Option<StringOrArray>,
}

/// Autoupdate architecture.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoopAutoupdateArchitecture {
    #[serde(rename = "32bit")]
    pub x32: Option<ScoopAutoupdateArchEntry>,
    #[serde(rename = "64bit")]
    pub x64: Option<ScoopAutoupdateArchEntry>,
    pub arm64: Option<ScoopAutoupdateArchEntry>,
}

impl ScoopAutoupdateArchitecture {
    pub fn get_entry(&self, arch: &str) -> Option<&ScoopAutoupdateArchEntry> {
        match arch {
            "32bit" => self.x32.as_ref(),
            "64bit" => self.x64.as_ref(),
            "arm64" => self.arm64.as_ref(),
            _ => None,
        }
    }
}

/// Autoupdate configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoopAutoupdate {
    pub url: Option<StringOrArray>,
    pub hash: Option<ScoopAutoupdateHash>,
    pub extract_dir: Option<StringOrArray>,
    pub architecture: Option<ScoopAutoupdateArchitecture>,
}

/// A Scoop app manifest.
/// https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests
#[derive(Debug, Clone, Deserialize)]
pub struct ScoopManifest {
    pub version: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<serde_json::Value>,
    pub url: Option<StringOrArray>,
    pub hash: Option<StringOrArray>,
    pub extract_dir: Option<StringOrArray>,
    pub architecture: Option<ScoopArchitecture>,
    pub bin: Option<ScoopBin>,
    pub env_add_path: Option<StringOrArray>,
    pub env_set: Option<HashMap<String, String>>,
    pub persist: Option<StringOrArray>,
    pub autoupdate: Option<ScoopAutoupdate>,
}

impl ScoopManifest {
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

    /// Get the SHA256 hash for a given architecture.
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
        if let Some(architecture) = &self.architecture
            && let Some(entry) = architecture.get_entry(arch)
            && let Some(bin) = &entry.bin
        {
            return bin.0.clone();
        }
        self.bin.as_ref().map(|b| b.0.clone()).unwrap_or_default()
    }

    /// Get env_add_path entries for a given architecture.
    pub fn get_env_add_paths_for_arch(&self, arch: &str) -> Vec<String> {
        if let Some(architecture) = &self.architecture
            && let Some(entry) = architecture.get_entry(arch)
            && let Some(paths) = &entry.env_add_path
        {
            return paths.0.clone();
        }
        self.env_add_path
            .as_ref()
            .map(|p| p.0.clone())
            .unwrap_or_default()
    }

    /// Substitute version placeholders in a URL template.
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

    /// Substitute version placeholders, also resolving `$baseurl` from the manifest's
    /// current download URL.
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

    /// Resolve the autoupdate extract_dir for a given version and architecture.
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
