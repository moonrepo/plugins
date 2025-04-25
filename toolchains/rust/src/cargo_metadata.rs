use serde::Deserialize;
use std::path::PathBuf;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct CargoMetadata {
    pub packages: Vec<PackageMetadata>,
    pub target_directory: PathBuf,
    pub workspace_root: PathBuf,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub targets: Vec<PackageTarget>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct PackageTarget {
    pub name: String,
    pub kind: Vec<PackageTargetKind>,
    pub crate_types: Vec<PackageTargetCrateType>,
    pub src_path: PathBuf,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PackageTargetKind {
    Bin,
    Lib,
    Test,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PackageTargetCrateType {
    Bin,
    Lib,
    RLib,
    DyLib,
    CdyLib,
    StaticLib,
    #[serde(rename = "proc-macro")]
    ProcMacro,
}
