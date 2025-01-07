// https://nx.dev/reference/nx-json
// https://github.com/nrwl/nx/blob/master/packages/nx/schemas/nx-schema.json

#![allow(dead_code)]

use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::json::JsonValue;

/// Only fields that are compatible with moon are documented,
/// anything else is ignored!

#[derive(Deserialize)]
#[serde(untagged)]
pub enum StringOrList {
    List(Vec<String>),
    String(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum NxDependsOn {
    Object {
        dependencies: Option<bool>,
        target: String,
        params: Option<String>,
        projects: Option<StringOrList>,
    },
    String(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum NxInput {
    Dep {
        dependencies: Option<bool>,
        projects: Option<StringOrList>,
        input: String,
    },
    #[serde(rename_all = "camelCase")]
    DepOutput {
        dependent_tasks_output_files: String,
        transitive: Option<bool>,
    },
    #[serde(rename_all = "camelCase")]
    External {
        external_dependencies: Vec<String>,
    },
    Env {
        env: String,
    },
    Fileset {
        fileset: String, // path, glob
    },
    Runtime {
        runtime: String, // command line
    },
    Source(String), // path, glob, group reference
}

pub type NxNamedInputs = FxHashMap<String, Vec<NxInput>>;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxAffected {
    pub default_base: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxTargetOptions {
    pub cache: Option<bool>,
    pub command: Option<String>,
    pub configurations: Option<FxHashMap<String, FxHashMap<String, JsonValue>>>,
    pub depends_on: Option<Vec<NxDependsOn>>,
    pub default_configuration: Option<String>,
    pub executor: Option<String>,
    pub inputs: Option<Vec<NxInput>>,
    pub options: Option<FxHashMap<String, JsonValue>>,
    pub outputs: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxWorkspaceLayout {
    pub apps_dir: Option<String>,
    pub libs_dir: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxJson {
    pub affected: Option<NxAffected>,
    pub default_base: Option<String>,
    pub named_inputs: Option<NxNamedInputs>,
    pub target_defaults: Option<FxHashMap<String, NxTargetOptions>>,
    pub workspace_layout: Option<NxWorkspaceLayout>,
    // Not supported:
    // implicitDependencies, tasksRunnerOptions, release, generators,
    // plugins, defaultProject, nxCloud*, parallel, cacheDirectory, useDaemonProcess
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxWorkspaceJson {
    pub projects: FxHashMap<String, String>,
}
