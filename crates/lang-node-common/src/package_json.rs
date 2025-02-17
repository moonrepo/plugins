use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct VoltaField {
    pub node: Option<String>,
    pub npm: Option<String>,
    pub pnpm: Option<String>,
    pub yarn: Option<String>,
    // Special
    pub extends: Option<String>,
}
