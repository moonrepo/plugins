use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum NodeDistLTS {
    Name(String),
    State(bool),
}

#[derive(Deserialize)]
pub struct NodeDistVersion {
    pub files: Vec<String>,
    pub lts: NodeDistLTS,
    pub npm: Option<String>, // No v prefix
    pub version: String,     // With v prefix
}
