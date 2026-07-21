// https://github.com/foojayio/discoapi
// https://sdkman.io/jdks/

use proto_pdk::{AnyResult, get_plugin_id};
use schematic::{ConfigEnum, derive_enum};

// Note: Our configuration (and proto itself) use kebab-case for
// variant values (for community compatibility), but Foojay uses
// snake_case. To support both patterns, we default to kebab-case,
// and then use serde aliases for any variants that require
// snake_case as well.
//
// Additionally, many of the serde aliases are to support SDKMAN!

// https://github.com/foojayio/discoapi/blob/main/src/main/java/io/foojay/api/pkg/Distro.java
derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum Distribution {
        Aoj,
        #[serde(alias = "aoj_openj9", alias = "aojopenj9")]
        AojOpenj9,
        Bisheng,
        #[serde(alias = "amazon", alias = "amzn")]
        Corretto,
        Debian,
        #[serde(alias = "dragon", alias = "alibaba", alias = "albba")]
        Dragonwell,
        #[serde(alias = "gluon_graalvm", alias = "gluon")]
        GluonGraalvm,
        #[serde(alias = "graal")]
        Graalvm,
        #[serde(alias = "graalvm_ce8", alias = "graalvmce8")]
        GraalvmCe8,
        #[serde(alias = "graalvm_ce11", alias = "graalvmce11")]
        GraalvmCe11,
        #[serde(alias = "graalvm_ce16", alias = "graalvmce16")]
        GraalvmCe16,
        #[serde(alias = "graalvm_ce17", alias = "graalvmce17")]
        GraalvmCe17,
        #[serde(alias = "graalvm_ce19", alias = "graalvmce19")]
        GraalvmCe19,
        #[serde(alias = "graalvm_ce20", alias = "graalvmce20")]
        GraalvmCe20,
        #[serde(alias = "graalvm_community", alias = "graalce")]
        GraalvmCommunity,
        Jetbrains,
        #[serde(alias = "tencent")]
        Kona,
        #[serde(alias = "librca")]
        Liberica,
        #[serde(alias = "liberica_native", alias = "nik")]
        LibericaNative,
        Mandrel,
        #[serde(alias = "ms")]
        Microsoft,
        #[serde(alias = "ojdk_build", alias = "ojdkbuild")]
        OjdkBuild,
        #[serde(alias = "open_logic", alias = "openlogic")]
        OpenLogic,
        #[default]
        #[serde(
            alias = "oracle_open_jdk",
            alias = "open_jdk",
            alias = "openjdk",
            alias = "open"
        )]
        OpenJdk,
        Oracle,
        Redhat,
        #[serde(
            alias = "sap_machine",
            alias = "sapmachine",
            alias = "sapmchn",
            alias = "sap"
        )]
        SapMachine,
        #[serde(alias = "sem")]
        Semeru,
        #[serde(alias = "semeru_certified")]
        SemeruCertified,
        #[serde(alias = "tem")]
        Temurin,
        Trava,
        Zulu,
        #[serde(alias = "zulu_prime", alias = "zuluprime")]
        ZuluPrime,
    }
);

impl Distribution {
    /// Parse from a string, including every serde alias. The generated
    /// `FromStr` only honors the first alias of each variant, while the
    /// serde deserializer honors them all (foojay and SDKMAN values).
    pub fn from_value(value: &str) -> AnyResult<Self> {
        use serde::Deserialize;

        Ok(Self::deserialize(serde::de::value::StrDeserializer::<
            serde::de::value::Error,
        >::new(value))?)
    }

    pub fn to_query_param(&self) -> String {
        match self {
            Distribution::OpenLogic => "openlogic".into(),
            Distribution::OpenJdk => "oracle_open_jdk".into(),
            _ => self.to_string().replace('-', "_"),
        }
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
