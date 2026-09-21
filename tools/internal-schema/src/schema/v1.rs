use super::{PlatformMapper, VERSION_REGEX};
use proto_pdk::{
    ExecutableConfig, HostArch, HostEnvironment, HostLibc, HostOS, PluginError, StringOrVec,
    UnresolvedVersionSpec, Version, VersionSpec,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PlatformMapperV1 {
    pub arch: HashMap<HostArch, String>,
    pub archs: Vec<HostArch>,
    pub archive_prefix: Option<String>,
    pub checksum_file: Option<String>,
    pub download_file: String,
    #[deprecated]
    pub exes_dir: Option<PathBuf>,
    pub exes_dirs: Vec<PathBuf>,
    pub exe_path: Option<PathBuf>,
    pub libc: HashMap<HostLibc, String>,
    #[deprecated]
    pub bin_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PluginSchema {
    pub description: Option<String>,
    pub repository_url: Option<String>,
    pub homepage_url: Option<String>,
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

impl ExecutableSchema {
    pub fn into_config(self) -> ExecutableConfig {
        ExecutableConfig {
            exe_path: self.exe_path,
            exe_link_path: self.exe_link_path,
            no_bin: self.no_bin,
            no_shim: self.no_shim,
            parent_exe_args: self.parent_exe_args,
            parent_exe_name: self.parent_exe_name,
            primary: self.primary,
            shim_before_args: self.shim_before_args.map(StringOrVec::Vec),
            shim_after_args: self.shim_after_args.map(StringOrVec::Vec),
            shim_env_vars: self.shim_env_vars.map(HashMap::from_iter),
            update_perms: false,
        }
    }
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
            version_pattern: VERSION_REGEX.into(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MetadataSchema {
    pub default_version: Option<UnresolvedVersionSpec>,
    pub plugin_version: Option<Version>,
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
pub struct SchemaV1 {
    pub name: String,
    #[serde(rename = "type")]
    pub type_of: SchemaType,
    pub plugin: PluginSchema,
    pub metadata: MetadataSchema,
    pub platform: HashMap<HostOS, PlatformMapperV1>,
    pub deprecations: Vec<String>,

    pub detect: DetectSchema,
    pub install: InstallSchema,
    pub packages: PackagesSchema,
    pub resolve: ResolveSchema,
}

impl SchemaV1 {
    pub fn get_platform(&self, env: &HostEnvironment) -> Result<PlatformMapper, PluginError> {
        let mut base = self.platform.get(&env.os);

        // Fallback to linux for other OSes
        if base.is_none() && env.os.is_bsd() {
            base = self.platform.get(&HostOS::Linux);
        }

        let base = base.ok_or_else(|| PluginError::UnsupportedOS {
            tool: self.name.clone(),
            os: env.os.to_rust_os(),
        })?;

        #[allow(deprecated)]
        let mut platform = PlatformMapper {
            arch: self.install.arch.clone(),
            archs: base.archs.clone(),
            archive_prefix: base.archive_prefix.clone(),
            checksum_file: base.checksum_file.clone(),
            download_file: Some(base.download_file.clone()),
            exes_dirs: Some(if let Some(dir) = &base.exes_dir {
                vec![dir.to_owned()]
            } else {
                base.exes_dirs.clone()
            }),
            exe_path: base.exe_path.clone().or(base.bin_path.clone()),
            libc: self.install.libc.clone(),
        };

        platform.arch.extend(base.arch.clone());
        platform.libc.extend(base.libc.clone());

        Ok(platform)
    }
}
