use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct DenoJson {
    #[serde(alias = "workspaces")]
    pub workspace: Option<DenoJsonWorkspace>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum DenoJsonWorkspace {
    Members(Vec<String>),
    Config { members: Vec<String> },
}

impl DenoJsonWorkspace {
    pub fn get_members(&self) -> &[String] {
        match self {
            Self::Members(members) => members,
            Self::Config { members } => members,
        }
    }
}
