use super::parse_version_spec;
use deno_lockfile::LockfileContent;
use moon_pdk::{AnyResult, VirtualPath};
use moon_pdk_api::{LockDependency, ParseLockOutput};
use serde::Deserialize;
use starbase_utils::json::{self, JsonValue};
use std::collections::BTreeMap;

// Reference: https://github.com/denoland/fresh/blob/main/deno.lock
pub fn parse_deno_lock(path: &VirtualPath, output: &mut ParseLockOutput) -> AnyResult<()> {
    let lockfile_content: JsonValue = json::read_file(path)?;
    let lockfile = LockfileContent::from_json(lockfile_content)?;

    for (key, value) in lockfile.packages.jsr {
        output
            .dependencies
            .entry(format!("jsr:{}", key.name))
            .or_default()
            .push(LockDependency {
                hash: Some(value.integrity),
                // Version is fully qualified
                version: parse_version_spec(key.version.to_string())?,
                ..Default::default()
            });
    }

    for (key, value) in lockfile.packages.npm {
        let (name, version) = parse_name_and_version(&key);

        output
            .dependencies
            .entry(format!("npm:{name}"))
            .or_default()
            .push(LockDependency {
                hash: value.integrity,
                // Version is fully qualified
                version: parse_version_spec(version)?,
                ..Default::default()
            });
    }

    Ok(())
}

fn parse_name_and_version(value: &str) -> (&str, &str) {
    // Remove parents: @babel/preset-react@7.27.1_@babel+core@7.28.3
    let value = value.split('_').next().unwrap();

    // Split on @ but preserve scope: @babel/preset-react@7.27.1
    if let Some(index) = value.rfind('@')
        && index != 0
    {
        return (&value[0..index], &value[index + 1..]);
    }

    // No version? Provide a fake value
    (value, "0.0.0")
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct DenoJson {
    pub tasks: BTreeMap<String, DenoJsonTask>,
    #[serde(alias = "workspaces")]
    pub workspace: Option<DenoJsonWorkspace>,
}

impl DenoJson {
    pub fn load_from(root: &VirtualPath) -> AnyResult<Self> {
        let config_file = root.join("deno.json");
        let configc_file = root.join("deno.jsonc");

        Ok(if config_file.exists() {
            json::read_file(config_file)?
        } else if configc_file.exists() {
            json::read_file(configc_file)?
        } else {
            Default::default()
        })
    }
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

#[derive(Deserialize)]
#[serde(untagged)]
pub enum DenoJsonTask {
    Command(String),
    Config {
        #[serde(default)]
        command: String,
        #[serde(default)]
        dependencies: Vec<String>,
    },
}

impl DenoJsonTask {
    pub fn get_command(&self) -> &String {
        match self {
            Self::Command(command) => command,
            Self::Config { command, .. } => command,
        }
    }

    pub fn get_dependencies(&self) -> &[String] {
        match self {
            Self::Command(_) => &[],
            Self::Config { dependencies, .. } => dependencies,
        }
    }
}
