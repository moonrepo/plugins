// https://github.com/foojayio/discoapi
// https://sdkman.io/jdks/

use proto_pdk::{AnyResult, anyhow, get_plugin_id};
use schematic::{ConfigEnum, derive_enum};
use serde::{Deserialize, Deserializer, de};
use std::fmt;

// Note: Our configuration (and proto itself) use kebab-case for
// variant values (for community compatibility), but Foojay uses
// snake_case. To support both patterns, we default to kebab-case,
// and then use serde aliases for any variants that require
// snake_case as well.
//
// Additionally, many of the serde aliases are to support SDKMAN!

// https://github.com/foojayio/discoapi/blob/main/src/main/java/io/foojay/api/pkg/Distro.java
#[derive(Clone, ConfigEnum, Default, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Distribution {
    Aoj,
    AojOpenj9,
    Bisheng,
    Corretto,
    Debian,
    Dragonwell,
    GluonGraalvm,
    Graalvm,
    GraalvmCe8,
    GraalvmCe11,
    GraalvmCe16,
    GraalvmCe17,
    GraalvmCe19,
    GraalvmCe20,
    GraalvmCommunity,
    Jetbrains,
    Kona,
    Liberica,
    LibericaNative,
    Mandrel,
    Microsoft,
    OjdkBuild,
    OpenLogic,
    #[default]
    #[serde(rename = "openjdk")]
    OpenJdk,
    Oracle,
    Redhat,
    SapMachine,
    Semeru,
    SemeruCertified,
    Temurin,
    Trava,
    Zulu,
    ZuluPrime,
}

impl Distribution {
    pub fn parse(value: &str) -> AnyResult<Self> {
        let value = value.to_lowercase();

        for var in Self::variants() {
            // Support SDKMAN and other shorthands!
            let matched = match var {
                Self::Corretto => value == "amazon" || value == "amzn",
                Self::Dragonwell => value == "dragon" || value == "alibaba" || value == "albba",
                Self::GluonGraalvm => value == "gluon",
                Self::Graalvm => value == "graal",
                Self::GraalvmCommunity => value == "graalce",
                Self::Kona => value == "tencent",
                Self::Liberica => value == "librca",
                Self::LibericaNative => value == "nik",
                Self::Microsoft => value == "msoft" || value == "ms",
                Self::OpenJdk => {
                    value == "oracle_open_jdk"
                        || value == "open-jdk"
                        || value == "open_jdk"
                        || value == "open"
                }
                Self::SapMachine => value == "sapmchn" || value == "sap",
                Self::Semeru => value == "sem",
                Self::Temurin => value == "tem",
                _ => false,
            };

            if matched {
                return Ok(var);
            }

            let kebab_case = var.to_string().to_lowercase();
            let snake_case = kebab_case.replace('-', "_");
            let no_case = kebab_case.replace('-', "");

            if value == kebab_case || value == snake_case || value == no_case {
                return Ok(var);
            }
        }

        Err(anyhow!("Unknown distribution vendor {value}"))
    }

    pub fn to_query_param(&self) -> String {
        match self {
            Distribution::OpenLogic => "openlogic".into(),
            Distribution::OpenJdk => "oracle_open_jdk".into(),
            _ => self.to_string().replace('-', "_"),
        }
    }
}

// We need a custom deserializer to support all the different values/aliases/etc
impl<'de> Deserialize<'de> for Distribution {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Distribution;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing a vendor distribution")
            }

            fn visit_str<E>(self, value: &str) -> Result<Distribution, E>
            where
                E: de::Error,
            {
                Distribution::parse(value).map_err(|error| de::Error::custom(error.to_string()))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum ArchiveType {
        Apk,
        Cab,
        Deb,
        Dmg,
        Exe,
        Msi,
        Pkg,
        Rpm,
        #[default]
        Tar,
        #[serde(alias = "tar.gz")]
        TarGz,
        #[serde(alias = "tar.xz")]
        TarXz,
        #[serde(alias = "tar.Z")]
        TarZ,
        Zip,
    }
);

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum PackageType {
        #[default]
        Jdk,
        Jre,
    }
);

impl PackageType {
    pub fn detect() -> AnyResult<Self> {
        let id = get_plugin_id()?;

        Ok(if id == "jre" { Self::Jre } else { Self::Jdk })
    }
}

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum ReleaseType {
        #[default]
        #[serde(alias = "ga")]
        GeneralAvailability,
        #[serde(alias = "ea")]
        EarlyAccess,
    }
);

impl ReleaseType {
    pub fn to_query_param(&self) -> String {
        match self {
            Self::GeneralAvailability => "ga".into(),
            Self::EarlyAccess => "ea".into(),
        }
    }
}

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum LibcType {
        #[default]
        #[serde(alias = "c_std_lib")]
        CStdLib,
        Glibc,
        Libc,
        Musl,
    }
);

#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct JavaToolConfig {
    pub api_url: String,
    pub release_type: ReleaseType,
}

impl Default for JavaToolConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.foojay.io/disco/v3.0".into(),
            release_type: ReleaseType::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod distribution {
        use super::*;

        #[test]
        fn parses_canonical_kebab_case() {
            assert_eq!(
                Distribution::parse("temurin").unwrap(),
                Distribution::Temurin
            );
            assert_eq!(
                Distribution::parse("openjdk").unwrap(),
                Distribution::OpenJdk
            );
            assert_eq!(
                Distribution::parse("sap-machine").unwrap(),
                Distribution::SapMachine
            );
            assert_eq!(
                Distribution::parse("liberica-native").unwrap(),
                Distribution::LibericaNative
            );
            assert_eq!(
                Distribution::parse("graalvm-community").unwrap(),
                Distribution::GraalvmCommunity
            );
            assert_eq!(
                Distribution::parse("open-logic").unwrap(),
                Distribution::OpenLogic
            );
        }

        #[test]
        fn parses_foojay_snake_case() {
            assert_eq!(
                Distribution::parse("sap_machine").unwrap(),
                Distribution::SapMachine
            );
            assert_eq!(
                Distribution::parse("liberica_native").unwrap(),
                Distribution::LibericaNative
            );
            assert_eq!(
                Distribution::parse("graalvm_community").unwrap(),
                Distribution::GraalvmCommunity
            );
            assert_eq!(
                Distribution::parse("aoj_openj9").unwrap(),
                Distribution::AojOpenj9
            );
        }

        #[test]
        fn parses_concatenated_no_case() {
            assert_eq!(
                Distribution::parse("sapmachine").unwrap(),
                Distribution::SapMachine
            );
            assert_eq!(
                Distribution::parse("libericanative").unwrap(),
                Distribution::LibericaNative
            );
            assert_eq!(
                Distribution::parse("openlogic").unwrap(),
                Distribution::OpenLogic
            );
        }

        #[test]
        fn parses_sdkman_shorthands() {
            for (value, dist) in [
                ("tem", Distribution::Temurin),
                ("amzn", Distribution::Corretto),
                ("amazon", Distribution::Corretto),
                ("librca", Distribution::Liberica),
                ("nik", Distribution::LibericaNative),
                ("ms", Distribution::Microsoft),
                ("msoft", Distribution::Microsoft),
                ("sem", Distribution::Semeru),
                ("sapmchn", Distribution::SapMachine),
                ("sap", Distribution::SapMachine),
                ("graalce", Distribution::GraalvmCommunity),
                ("graal", Distribution::Graalvm),
                ("gluon", Distribution::GluonGraalvm),
                ("dragon", Distribution::Dragonwell),
                ("alibaba", Distribution::Dragonwell),
                ("albba", Distribution::Dragonwell),
                ("tencent", Distribution::Kona),
                ("open", Distribution::OpenJdk),
                ("oracle_open_jdk", Distribution::OpenJdk),
            ] {
                assert_eq!(Distribution::parse(value).unwrap(), dist, "for {value}");
            }
        }

        #[test]
        fn parses_case_insensitively() {
            assert_eq!(
                Distribution::parse("TEMURIN").unwrap(),
                Distribution::Temurin
            );
            assert_eq!(Distribution::parse("Zulu").unwrap(), Distribution::Zulu);
            assert_eq!(
                Distribution::parse("SAP_MACHINE").unwrap(),
                Distribution::SapMachine
            );
            assert_eq!(Distribution::parse("TEM").unwrap(), Distribution::Temurin);
        }

        #[test]
        fn defaults_to_openjdk() {
            assert_eq!(Distribution::default(), Distribution::OpenJdk);
        }

        #[test]
        fn errors_on_unknown() {
            assert!(Distribution::parse("foobar").is_err());
            assert!(Distribution::parse("eliya").is_err());
        }

        #[test]
        fn query_param_uses_foojay_spelling() {
            // Most map by replacing dashes with underscores
            assert_eq!(Distribution::Temurin.to_query_param(), "temurin");
            assert_eq!(Distribution::SapMachine.to_query_param(), "sap_machine");
            assert_eq!(
                Distribution::LibericaNative.to_query_param(),
                "liberica_native"
            );
            // But these two are special cased for foojay
            assert_eq!(Distribution::OpenJdk.to_query_param(), "oracle_open_jdk");
            assert_eq!(Distribution::OpenLogic.to_query_param(), "openlogic");
        }
    }
}
