use crate::{
    config::{Distribution, PackageType},
    version::to_java_version,
};
use proto_pdk::{AnyResult, UnresolvedVersionSpec, VersionSpec};
use std::str::FromStr;

#[derive(Default)]
pub struct JavaContext {
    pub distribution: Distribution,
    pub package: PackageType,
    pub spec: VersionSpec,
    pub full_version: String,
    pub short_version: String,
}

impl JavaContext {
    pub fn detect(base_spec: &VersionSpec) -> AnyResult<Self> {
        let mut distribution = Distribution::default();
        let mut spec = VersionSpec::default();

        if let VersionSpec::Version(version) = &base_spec {
            if let Some(scope) = &version.scope {
                distribution = Distribution::from_str(scope)?;

                let mut version = version.to_owned();
                version.scope = None;

                spec = VersionSpec::Version(version);
            } else {
                spec = base_spec.to_owned()
            }
        }

        let package = PackageType::detect()?;

        Ok(Self {
            full_version: spec.to_string(),
            short_version: to_java_version(&spec),
            distribution,
            package,
            spec,
        })
    }

    pub fn detect_from_unresolved(base_spec: &UnresolvedVersionSpec) -> AnyResult<Self> {
        let mut distribution = Distribution::default();

        match base_spec {
            UnresolvedVersionSpec::Requirement(req) => {
                if let Some(scope) = &req.scope {
                    distribution = Distribution::from_str(scope)?;
                }
            }
            UnresolvedVersionSpec::Version(version) => {
                if let Some(scope) = &version.scope {
                    distribution = Distribution::from_str(scope)?;
                }
            }
            _ => {}
        };

        let package = PackageType::detect()?;

        Ok(Self {
            distribution,
            package,
            ..Default::default()
        })
    }
}
