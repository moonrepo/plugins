use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum NodeDistLTS {
    State(bool),
    Name(String),
}

#[derive(Deserialize)]
pub struct NodeDistVersion {
    pub files: Vec<String>,
    pub lts: NodeDistLTS,
    pub npm: Option<String>, // No v prefix
    pub version: String,     // With v prefix
}
