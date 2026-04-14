#![allow(dead_code)]

use crate::config::DEFAULT_REGISTRY;
use crate::yarn_compat::*;
use npmrc_config_rs::{
    Credentials, LoadOptions, NpmrcConfig, nerf_dart, registry::parse_registry_url,
};
use proto_pdk::{
    AnyResult, UnresolvedVersionSpec, Version, VersionSpec, VirtualPath, get_plugin_id,
};
use rustc_hash::FxHashMap;
use starbase_utils::{fs::find_upwards, yaml};
use std::fmt;

#[derive(PartialEq)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
}

impl PackageManager {
    pub fn detect() -> AnyResult<PackageManager> {
        let id = get_plugin_id()?;

        Ok(if id.to_lowercase().contains("yarn") {
            PackageManager::Yarn
        } else if id.to_lowercase().contains("pnpm") {
            PackageManager::Pnpm
        } else {
            PackageManager::Npm
        })
    }

    pub fn get_package_name(&self, version: impl AsRef<UnresolvedVersionSpec>) -> String {
        let version = version.as_ref();

        if matches!(self, PackageManager::Yarn) {
            if let UnresolvedVersionSpec::Semantic(inner) = &version {
                // Version 2.4.3 was published to the wrong package. It should
                // have been published to `@yarnpkg/cli-dist` but was published
                // to `yarn`. So... we need to manually fix it.
                // https://www.npmjs.com/package/yarn?activeTab=versions
                if inner.major == 2 && inner.minor == 4 && inner.patch == 3 {
                    return "yarn".into();
                }
            }

            if self.is_yarn_berry(version) {
                return "@yarnpkg/cli-dist".into();
            }
        }

        self.to_string()
    }

    pub fn get_http_headers(
        &self,
        registry_url: &str,
        working_dir: &VirtualPath,
    ) -> AnyResult<FxHashMap<String, String>> {
        let mut headers = FxHashMap::default();
        let url = parse_registry_url(registry_url)?;

        let credentials = match self {
            Self::Npm | Self::Pnpm => {
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
            Self::Yarn => {
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

    pub fn is_pnpm_11(&self, version: impl AsRef<VersionSpec>) -> bool {
        let version_11 = Version::parse("11.0.0-rc.0").unwrap();

        // matches!(self, PackageManager::Pnpm)
        //     && match version.as_ref() {
        //         UnresolvedVersionSpec::Semantic(ver) => ver.0 >= version_11,
        //         UnresolvedVersionSpec::Req(req) => req.matches(&version_11),
        //         UnresolvedVersionSpec::ReqAny(reqs) => {
        //             reqs.iter().any(|req| req.matches(&version_11))
        //         }
        //         _ => false,
        //     }

        matches!(self, PackageManager::Pnpm)
            && match version.as_ref() {
                VersionSpec::Semantic(ver) => ver.0 >= version_11,
                _ => false,
            }
    }

    pub fn is_yarn_classic(&self, version: impl AsRef<UnresolvedVersionSpec>) -> bool {
        matches!(self, PackageManager::Yarn)
            && match version.as_ref() {
                UnresolvedVersionSpec::Alias(alias) => alias == "legacy" || alias == "classic",
                UnresolvedVersionSpec::Semantic(ver) => ver.major == 1,
                UnresolvedVersionSpec::Req(req) => req.comparators.iter().any(|c| c.major == 1),
                _ => false,
            }
    }

    pub fn is_yarn_berry(&self, version: impl AsRef<UnresolvedVersionSpec>) -> bool {
        matches!(self, PackageManager::Yarn)
            && match version.as_ref() {
                UnresolvedVersionSpec::Alias(alias) => alias == "berry" || alias == "latest",
                UnresolvedVersionSpec::Semantic(ver) => ver.major > 1,
                UnresolvedVersionSpec::Req(req) => req.comparators.iter().any(|c| c.major > 1),
                _ => false,
            }
    }
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageManager::Npm => write!(f, "npm"),
            PackageManager::Pnpm => write!(f, "pnpm"),
            PackageManager::Yarn => write!(f, "yarn"),
        }
    }
}
