use rustc_hash::FxHashMap;
use serde::Deserialize;

// Only type fields we actually need!

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub alias: Option<String>, // package.json name
    pub id: String,
    pub source: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGraph {
    pub projects: FxHashMap<String, Project>,
}
