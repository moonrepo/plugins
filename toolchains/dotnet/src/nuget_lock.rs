use serde::Deserialize;
use std::collections::BTreeMap;

/// A NuGet `packages.lock.json` file.
///
/// Shape: `{ "version": 1, "dependencies": { "<tfm>": { "<PackageName>":
/// { "type", "requested", "resolved", "contentHash", ... } } } }`
#[derive(Debug, Default, Deserialize)]
pub struct NugetLockFile {
    #[serde(default)]
    pub version: u32,

    #[serde(default)]
    pub dependencies: BTreeMap<String, BTreeMap<String, NugetLockEntry>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NugetLockEntry {
    /// "Direct", "Transitive", "Project", or "CentralTransitive".
    #[serde(default, rename = "type")]
    pub dep_type: String,

    #[serde(default)]
    pub requested: Option<String>,

    #[serde(default)]
    pub resolved: Option<String>,

    #[serde(default)]
    pub content_hash: Option<String>,
}

pub fn parse_lock_file(content: &str) -> Result<NugetLockFile, serde_json::Error> {
    serde_json::from_str(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "version": 1,
  "dependencies": {
    "net8.0": {
      "Newtonsoft.Json": {
        "type": "Direct",
        "requested": "[13.0.3, )",
        "resolved": "13.0.3",
        "contentHash": "HrC5BXdl00IP9zeV+0Z848QWPAoCr9P3bDEZguI+gkLcBKAOxix/tLEAAHC+UvDNPv4a2d18lOReHMOagPa+zQ=="
      },
      "App": {
        "type": "Project",
        "dependencies": {
          "Lib": "[1.0.0, )"
        }
      }
    },
    "net9.0": {
      "Newtonsoft.Json": {
        "type": "Direct",
        "requested": "[13.0.3, )",
        "resolved": "13.0.3",
        "contentHash": "HrC5BXdl00IP9zeV+0Z848QWPAoCr9P3bDEZguI+gkLcBKAOxix/tLEAAHC+UvDNPv4a2d18lOReHMOagPa+zQ=="
      }
    }
  }
}"#;

    #[test]
    fn parses_lock_file() {
        let lock = parse_lock_file(SAMPLE).unwrap();

        assert_eq!(lock.version, 1);
        assert_eq!(lock.dependencies.len(), 2);

        let net8 = &lock.dependencies["net8.0"];
        let newtonsoft = &net8["Newtonsoft.Json"];

        assert_eq!(newtonsoft.dep_type, "Direct");
        assert_eq!(newtonsoft.requested.as_deref(), Some("[13.0.3, )"));
        assert_eq!(newtonsoft.resolved.as_deref(), Some("13.0.3"));
        assert!(
            newtonsoft
                .content_hash
                .as_deref()
                .unwrap()
                .starts_with("HrC5")
        );

        // Project-type entries carry no resolved version/hash.
        let project = &net8["App"];
        assert_eq!(project.dep_type, "Project");
        assert!(project.resolved.is_none());
    }

    #[test]
    fn tolerates_empty_file_shape() {
        let lock = parse_lock_file("{}").unwrap();

        assert_eq!(lock.version, 0);
        assert!(lock.dependencies.is_empty());
    }
}
