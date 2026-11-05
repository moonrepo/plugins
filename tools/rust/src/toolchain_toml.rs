use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ToolchainSection {
    pub channel: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ToolchainToml {
    pub toolchain: ToolchainSection,
}
