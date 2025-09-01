use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoWorkspaceJson {
    #[serde(alias = "workspaces")]
    pub workspace: Option<Vec<String>>,
}
