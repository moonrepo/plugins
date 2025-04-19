use proto_pdk::{HostArch, HostLibc, HostOS, UnresolvedVersionSpec, VersionSpec};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PlatformMapper {
    pub archs: Vec<HostArch>,
    pub archive_prefix: Option<String>,
    pub checksum_file: Option<String>,
    pub download_file: String,
    #[deprecated]
    pub exes_dir: Option<PathBuf>,
    pub exes_dirs: Vec<PathBuf>,
    pub exe_path: Option<PathBuf>,
    #[deprecated]
    pub bin_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DetectSchema {
    pub ignore: Vec<String>,
    pub version_files: Vec<String>,
}

// Keep in sync with the `ExecutableConfig` shape!
// We had to create another struct so that we can serde rename...
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ExecutableSchema {
    pub exe_path: Option<PathBuf>,
    pub exe_link_path: Option<PathBuf>,
    pub no_bin: bool,
    pub no_shim: bool,
    pub parent_exe_args: Vec<String>,
    pub parent_exe_name: Option<String>,
    pub primary: bool,
    pub shim_before_args: Option<Vec<String>>,
    pub shim_after_args: Option<Vec<String>>,
    pub shim_env_vars: Option<HashMap<String, String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct InstallSchema {
    pub arch: HashMap<HostArch, String>,
    pub libc: HashMap<HostLibc, String>,
    pub checksum_public_key: Option<String>,
    pub checksum_url: Option<String>,
    pub checksum_url_canary: Option<String>,
    pub download_url: String,
    pub download_url_canary: Option<String>,
    pub exes: HashMap<String, ExecutableSchema>,

    // Primary
    #[deprecated]
    pub primary: Option<ExecutableSchema>,
    pub no_bin: Option<bool>,
    pub no_shim: Option<bool>,

    // Secondary
    #[deprecated]
    pub secondary: HashMap<String, ExecutableSchema>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PackagesSchema {
    pub globals_lookup_dirs: Vec<String>,
    pub globals_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ResolveSchema {
    pub aliases: HashMap<String, UnresolvedVersionSpec>,
    pub versions: Vec<VersionSpec>,
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
            aliases: HashMap::new(),
            manifest_url: None,
            manifest_version_key: "version".to_string(),
            git_url: None,
            git_tag_pattern: None,
            versions: vec![],
            version_pattern:
                r"^v?((?<major>[0-9]+)\.(?<minor>[0-9]+)\.(?<patch>[0-9]+)(?<pre>-[0-9a-zA-Z\.]+)?(?<build>\+[-0-9a-zA-Z\.]+)?)$"
                    .to_string(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MetadataSchema {
    pub requires: Vec<String>,
    pub self_upgrade_commands: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaType {
    #[serde(alias = "cli")]
    CommandLine,
    #[serde(alias = "package-manager")]
    DependencyManager,
    #[default]
    Language,
    VersionManager,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Schema {
    pub name: String,
    #[serde(rename = "type")]
    pub type_of: SchemaType,
    pub metadata: MetadataSchema,
    pub platform: HashMap<HostOS, PlatformMapper>,
    pub deprecations: Vec<String>,

    pub detect: DetectSchema,
    pub install: InstallSchema,
    pub packages: PackagesSchema,
    pub resolve: ResolveSchema,
}
