use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

// Only type fields we actually need!

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub alias: Option<String>, // package.json name
    pub id: String,
    pub source: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGraph {
    pub projects: FxHashMap<String, Project>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_plugin_serialization_interface() {
        let mut projects = FxHashMap::default();
        projects.insert(
            "app".to_string(),
            Project {
                alias: Some("@scope/app".to_string()),
                id: "app".to_string(),
                source: "apps/app".to_string(),
            },
        );

        let graph = ProjectGraph { projects };

        // Test serialization to JSON (host -> plugin interface)
        let serialized = serde_json::to_string(&graph).expect("Serialization failed");
        assert!(serialized.contains("\"id\":\"app\""));
        assert!(serialized.contains("\"alias\":\"@scope/app\""));

        // Test deserialization from JSON (plugin -> host interface)
        let deserialized: ProjectGraph =
            serde_json::from_str(&serialized).expect("Deserialization failed");
        assert_eq!(graph, deserialized);
    }
}

