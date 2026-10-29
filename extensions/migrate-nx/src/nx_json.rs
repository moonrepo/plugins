// https://nx.dev/reference/nx-json
// https://github.com/nrwl/nx/blob/master/packages/nx/schemas/nx-schema.json

#![allow(dead_code)]

use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::json::JsonValue;

/// Only fields that are compatible with moon are documented,
/// anything else is ignored!

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum StringOrList {
    List(Vec<String>),
    String(String),
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum NxDependsOn {
    #[serde(rename_all = "camelCase")]
    Object {
        dependencies: Option<bool>,
        target: String,
        params: Option<String>,
        projects: Option<StringOrList>, // name, list of names, {self}, {dependencies}
        wait_for: Option<String>,       // started, ready
    },
    String(String),
}

#[derive(Clone, Deserialize)]
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
        dependencies: Option<bool>,
        fileset: String, // path, glob
    },
    Json {
        json: String, // path, glob
    },
    Runtime {
        runtime: String, // command line
    },
    #[serde(rename_all = "camelCase")]
    WorkingDirectory {
        working_directory: String, // relative, absolute
    },
    Source(String), // path, glob, group reference, spread
}

pub type NxNamedInputs = FxHashMap<String, Vec<NxInput>>;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxAffected {
    pub default_base: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
pub enum NxTargetConfiguration {
    Options(FxHashMap<String, JsonValue>),
    Spread(bool),
}

#[derive(Clone, Default, Deserialize)]
pub struct NxTargetMetadata {
    pub description: Option<String>,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NxTargetOptions {
    pub cache: Option<bool>,
    pub command: Option<String>,
    pub configurations: Option<FxHashMap<String, NxTargetConfiguration>>,
    pub continuous: Option<bool>,
    pub depends_on: Option<Vec<NxDependsOn>>,
    pub default_configuration: Option<String>,
    pub executor: Option<String>,
    pub inputs: Option<Vec<NxInput>>,
    pub metadata: Option<NxTargetMetadata>,
    pub options: Option<FxHashMap<String, JsonValue>>,
    pub outputs: Option<Vec<String>>,
}

impl NxTargetOptions {
    // Each setting replaces the inherited one, unless using the spread token
    pub fn inherit(&mut self, defaults: &NxTargetOptions) {
        self.cache = self.cache.or(defaults.cache);
        self.command = self.command.take().or_else(|| defaults.command.clone());
        self.configurations = self
            .configurations
            .take()
            .or_else(|| defaults.configurations.clone());
        self.continuous = self.continuous.or(defaults.continuous);
        self.depends_on = self
            .depends_on
            .take()
            .or_else(|| defaults.depends_on.clone());
        self.default_configuration = self
            .default_configuration
            .take()
            .or_else(|| defaults.default_configuration.clone());
        self.executor = self.executor.take().or_else(|| defaults.executor.clone());
        self.inputs = self.inputs.take().or_else(|| defaults.inputs.clone());
        self.metadata = self.metadata.take().or_else(|| defaults.metadata.clone());
        self.options = self.options.take().or_else(|| defaults.options.clone());
        self.outputs = self.outputs.take().or_else(|| defaults.outputs.clone());
    }
}

#[derive(Default, Deserialize)]
pub struct NxTargetDefaultsFilter {
    pub executor: Option<String>,
    pub plugin: Option<String>,
    pub projects: Option<StringOrList>,
}

#[derive(Default, Deserialize)]
pub struct NxTargetDefaultsEntry {
    pub filter: Option<NxTargetDefaultsFilter>,
    #[serde(flatten)]
    pub target: NxTargetOptions,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum NxTargetDefaults {
    List(Vec<NxTargetDefaultsEntry>),
    Object(NxTargetOptions),
}

impl NxTargetDefaults {
    // Filtered entries depend on the project graph, so only the unfiltered entries are used,
    // with later entries overriding earlier ones
    pub fn resolve(self) -> Option<NxTargetOptions> {
        match self {
            Self::List(entries) => entries
                .into_iter()
                .filter(|entry| entry.filter.is_none())
                .map(|entry| entry.target)
                .reduce(|prev, mut next| {
                    next.inherit(&prev);
                    next
                }),
            Self::Object(target) => Some(target),
        }
    }
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
    pub target_defaults: Option<FxHashMap<String, NxTargetDefaults>>,
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
