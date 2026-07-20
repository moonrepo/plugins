#![allow(dead_code)]

use crate::config::DEFAULT_REGISTRY;
use crate::yarn_compat::*;
use npmrc_config_rs::{
    Credentials, LoadOptions, NpmrcConfig, nerf_dart, registry::parse_registry_url,
};
use proto_pdk::{AnyResult, VersionSpec, VirtualPath, get_plugin_id};
use rustc_hash::FxHashMap;
use starbase_utils::{fs::find_upwards, yaml};

#[derive(Debug, PartialEq)]
pub enum PackageManager {
    Npm,

    Pnpm,
    Pnpm11,

    Yarn1,
    Yarn2to5,
    Yarn6,
}

impl PackageManager {
    pub fn detect() -> AnyResult<PackageManager> {
        let id = get_plugin_id()?;

        Ok(if id.to_lowercase().contains("yarn") {
            Self::Yarn1
        } else if id.to_lowercase().contains("pnpm") {
            Self::Pnpm
        } else {
            Self::Npm
        })
    }

    pub fn detect_from_version(version: &VersionSpec) -> AnyResult<PackageManager> {
        let mut manager = Self::detect()?;

        if manager == Self::Pnpm {
            manager = match version {
                VersionSpec::Canary => Self::Pnpm11,
                VersionSpec::Alias(alias) => {
                    if alias == "latest" {
                        Self::Pnpm11
                    } else {
                        Self::Pnpm
                    }
                }
                VersionSpec::Version(version) => {
                    if version.major >= 11 {
                        Self::Pnpm11
                    } else {
                        Self::Pnpm
                    }
                }
            };
        } else if manager == Self::Yarn1 {
            manager = match version {
                VersionSpec::Canary => Self::Yarn6,
                VersionSpec::Alias(alias) => {
                    if alias == "classic" || alias == "legacy" {
                        Self::Yarn1
                    } else if alias == "rust" || alias == "zpm" {
                        Self::Yarn6
                    } else {
                        Self::Yarn2to5
                    }
                }
                VersionSpec::Version(version) => {
                    if version.major >= 6 {
                        Self::Yarn6
                    } else if version.major >= 2 {
                        Self::Yarn2to5
                    } else {
                        Self::Yarn1
                    }
                }
            };
        }

        Ok(manager)
    }

    pub fn is_npm(&self) -> bool {
        matches!(self, Self::Npm)
    }

    pub fn is_pnpm(&self) -> bool {
        matches!(self, Self::Pnpm | Self::Pnpm11)
    }

    pub fn is_yarn(&self) -> bool {
        matches!(self, Self::Yarn1 | Self::Yarn2to5 | Self::Yarn6)
    }

    pub fn get_bin_name(&self) -> String {
        match self {
            Self::Npm => "npm".into(),
            Self::Pnpm | Self::Pnpm11 => "pnpm".into(),
            Self::Yarn1 | Self::Yarn2to5 | Self::Yarn6 => "yarn".into(),
        }
    }

    pub fn get_package_name(&self) -> String {
        match self {
            Self::Yarn2to5 => "@yarnpkg/cli-dist".into(),
            _ => self.get_bin_name(),
        }
    }

    pub fn get_http_headers(
        &self,
        registry_url: &str,
        working_dir: &VirtualPath,
    ) -> AnyResult<FxHashMap<String, String>> {
        let mut headers = FxHashMap::default();
        let url = parse_registry_url(registry_url)?;

        let credentials = match self {
            Self::Npm | Self::Pnpm | Self::Pnpm11 => {
                let rc = NpmrcConfig::load_with_options(LoadOptions {
                    cwd: Some(working_dir.into()),
                    global_prefix: None,
                    user_config: Some("/userhome/.npmrc".into()),
                    skip_project: false,
                    skip_user: false,
                    skip_global: true,
                })?;

                rc.credentials_for(&url)
            }
            Self::Yarn1 | Self::Yarn2to5 | Self::Yarn6 => {
                if let Some(rc_path) = find_upwards(".yarnrc.yml", working_dir) {
                    let mut rc: YarnRcYaml = yaml::read_file(rc_path)?;
                    let registry_shorthand = nerf_dart(&url);
                    let registry_shorthand_without_slash = registry_shorthand.trim_end_matches('/');

                    let (token, basic) =
                        if let Some(config) = rc.npm_registries.remove(&registry_shorthand) {
                            (config.npm_auth_token, config.npm_auth_ident)
                        } else if let Some(config) =
                            rc.npm_registries.remove(registry_shorthand_without_slash)
                        {
                            (config.npm_auth_token, config.npm_auth_ident)
                        } else if rc.npm_registry_server.as_ref().is_none_or(|server| {
                            registry_url == server || registry_url == DEFAULT_REGISTRY
                        }) {
                            (rc.npm_auth_token, rc.npm_auth_ident)
                        } else {
                            (None, None)
                        };

                    if let Some(token) = token {
                        Some(Credentials::Token { token, cert: None })
                    } else if let Some(basic) = basic
                        && let Some((user, pass)) = basic.split_once(':')
                    {
                        Some(Credentials::BasicAuth {
                            username: user.into(),
                            password: pass.into(),
                            cert: None,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        // https://github.com/npm/registry/blob/main/docs/user/authentication.md
        if let Some(creds) = credentials {
            match &creds {
                Credentials::Token { token, .. } => {
                    headers.insert("Authorization".into(), format!("Bearer {token}"));
                }
                Credentials::BasicAuth { .. } | Credentials::LegacyAuth { .. } => {
                    if let Some(encoded) = creds.basic_auth_header() {
                        headers.insert("Authorization".into(), format!("Basic {encoded}"));
                    }
                }
                Credentials::ClientCertOnly(_) => {}
            };
        }

        Ok(headers)
    }
}
