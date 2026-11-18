use common::debug;
use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum NodeDistLTS {
    State(bool),
    Name(String),
    // ADR-0008.4
    Unknown(JsonValue),
}

impl Default for NodeDistLTS {
    fn default() -> Self {
        Self::State(false)
    }
}

// ADR-0008.3
#[derive(Deserialize)]
pub struct NodeDistVersion {
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub lts: NodeDistLTS,
    #[serde(default)]
    pub npm: Option<String>, // No v prefix
    pub version: String, // With v prefix
}

impl NodeDistVersion {
    /// Parse the entries of a nodejs.org index, like `index.json`, skipping
    /// the ones that can't be parsed (ADR-0008.5).
    pub fn parse_index(entries: Vec<JsonValue>) -> Vec<Self> {
        entries
            .into_iter()
            .filter_map(|entry| {
                let version = entry.get("version").cloned();

                serde_json::from_value(entry)
                    .inspect_err(|error| {
                        debug!("Skipping nodejs.org index entry {version:?}: {error}");
                    })
                    .ok()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn skips_entries_that_cant_be_parsed() {
        let versions = NodeDistVersion::parse_index(vec![
            json!({ "version": "v26.0.0", "files": ["osx-arm64-tar"], "lts": false }),
            json!({ "version": "v24.22.3", "lts": "Krypton", "npm": "11.13.0" }),
            json!({ "version": "v24.0.0", "lts": { "since": "2026-10-28" } }),
            json!({ "version": 27 }),
            json!({ "files": [] }),
        ]);

        assert_eq!(
            versions
                .iter()
                .map(|item| item.version.as_str())
                .collect::<Vec<_>>(),
            ["v26.0.0", "v24.22.3", "v24.0.0"]
        );
        assert!(matches!(&versions[1].lts, NodeDistLTS::Name(name) if name == "Krypton"));
        assert!(matches!(versions[2].lts, NodeDistLTS::Unknown(_)));
        assert!(versions[1].files.is_empty());
    }
}
