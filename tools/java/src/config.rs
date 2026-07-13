// https://github.com/foojayio/discoapi
use schematic::{ConfigEnum, derive_enum};

// Note: Our configuration (and proto itself) use kebab-case for
// variant values, but Foojay uses snake_case. To support both
// patterns, we default to kebab-case, and then use serde aliases
// for any variants that require snake_case as well.

derive_enum!(
    #[derive(ConfigEnum, Default)]
    pub enum Distribution {
        Aoj,
        #[serde(alias = "aoj_openj9")]
        AojOpenj9,
        Corretto,
        Dragonwell,
        #[serde(alias = "graalvm_ce8")]
        GraalvmCe8,
        #[serde(alias = "graalvm_ce11")]
        GraalvmCe11,
        #[serde(alias = "graalvm_ce16")]
        GraalvmCe16,
        Jetbrains,
        Liberica,
        #[serde(alias = "liberica_native")]
        LibericaNative,
        Mandrel,
        Microsoft,
        #[serde(alias = "ojdk_build")]
        OjdkBuild,
        Openlogic,
        Oracle,
        #[serde(alias = "oracle_open_jdk")]
        OracleOpenJdk,
        Redhat,
        #[serde(alias = "sap_machine")]
        SapMachine,
        Semeru,
        #[default]
        Temurin,
        Trava,
        Zulu,
        #[serde(alias = "zulu_prime")]
        ZuluPrime,
    }
);

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
    pub distribution: Distribution,
    pub package_type: PackageType,
    pub release_type: ReleaseType,
}

impl Default for JavaToolConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.foojay.io/disco/v3.0".into(),
            distribution: Distribution::default(),
            package_type: PackageType::default(),
            release_type: ReleaseType::default(),
        }
    }
}
