// `pyproject.toml`

use crate::config::PythonPackageManager;
#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::{HostLogInput, host_log};
use moon_pdk_api::{AnyResult, toml_config};
use pep508_rs::Requirement;
use serde::{Deserialize, Serialize};
use starbase_utils::toml::TomlValue;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use toolchain_common::debug;

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

// Python distribution names are compared using the normalized form from PEP 503.
pub fn normalize_distribution_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut last_was_separator = false;

    for ch in name.chars() {
        if matches!(ch, '-' | '_' | '.') {
            if !last_was_separator {
                normalized.push('-');
                last_was_separator = true;
            }
        } else {
            normalized.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        }
    }

    normalized
}

/// Resolve `.` and `..` in a path without touching the file system,
/// as WASM can't canonicalize, so `/a/b` + `../c` becomes `/a/c`.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();

    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }

    out
}

// ADR-0008.2: Only define the fields the plugin reads, so that a field we
// don't use, or a shape we don't know, can't fail the whole file.
// Workspaces: https://docs.astral.sh/uv/concepts/projects/workspaces/

toml_config!(PyProjectToml, PyProjectTomlInner);

impl PyProjectToml {
    #[allow(dead_code)]
    pub fn save_field(&self, _field: &str, _config: &mut TomlValue) -> AnyResult<()> {
        Ok(())
    }

    pub fn get_tool_poetry(&self) -> Option<&ToolPoetry> {
        self.tool.as_ref().and_then(|tool| tool.poetry.as_ref())
    }

    pub fn get_tool_uv(&self) -> Option<&ToolUv> {
        self.tool.as_ref().and_then(|tool| tool.uv.as_ref())
    }

    pub fn is_uv_workspace(&self) -> bool {
        self.get_tool_uv().is_some_and(|uv| uv.workspace.is_some())
    }

    /// Extract package members if the current manifest is a workspace.
    pub fn extract_members(&self, package_manager: PythonPackageManager) -> Option<Vec<String>> {
        if package_manager == PythonPackageManager::Uv
            && let Some(workspace) = self.get_tool_uv().and_then(|uv| uv.workspace.as_ref())
        {
            let mut members = vec![];

            if let Some(include) = &workspace.members {
                for inc in include {
                    members.push(inc.to_owned());
                }
            }

            if let Some(exclude) = &workspace.exclude {
                for ex in exclude {
                    members.push(format!("!{ex}"));
                }
            }

            if !members.is_empty() {
                return Some(members);
            }
        }

        None
    }

    /// `[project].dependencies`.
    pub fn get_dependencies(&self) -> Vec<Requirement> {
        match &self.project {
            Some(project) => self.parse_requirements(&project.dependencies),
            None => vec![],
        }
    }

    /// `[project].optional-dependencies`, for every extra.
    pub fn get_optional_dependencies(&self) -> Vec<Requirement> {
        match &self.project {
            Some(project) => project
                .optional_dependencies
                .values()
                .flat_map(|reqs| self.parse_requirements(reqs))
                .collect(),
            None => vec![],
        }
    }

    /// `[dependency-groups]` (PEP 735), for every group.
    pub fn get_dependency_groups(&self) -> Vec<Requirement> {
        self.dependency_groups
            .values()
            .flat_map(|entries| {
                // Included groups are read on their own anyways
                let reqs = entries
                    .iter()
                    .filter_map(|entry| match entry {
                        DependencyGroupEntry::Requirement(req) => Some(req.to_owned()),
                        DependencyGroupEntry::Other(_) => None,
                    })
                    .collect::<Vec<_>>();

                self.parse_requirements(&reqs)
            })
            .collect()
    }

    /// `[tool.uv].dev-dependencies`, the legacy form of the `dev` group.
    pub fn get_uv_dev_dependencies(&self) -> Vec<Requirement> {
        match self.get_tool_uv() {
            Some(uv) => self.parse_requirements(&uv.dev_dependencies),
            None => vec![],
        }
    }

    /// Whether the project is built by a backend, and so installed itself,
    /// as uv decides it for `uv sync`.
    pub fn has_build_system(&self) -> bool {
        self.build_system.is_some()
    }

    /// Whether `[project].dependencies` is provided by the build backend.
    pub fn has_dynamic_dependencies(&self) -> bool {
        self.project
            .as_ref()
            .is_some_and(|project| project.dynamic.iter().any(|field| field == "dependencies"))
    }

    // ADR-0008.5: Skip a requirement that can't be parsed, instead of failing the whole file
    fn parse_requirements(&self, values: &[String]) -> Vec<Requirement> {
        values
            .iter()
            .filter_map(|value| {
                Requirement::from_str(value)
                    .inspect_err(|error| {
                        debug!("Skipping the requirement {value} in {}: {error}", self.path)
                    })
                    .ok()
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PyProjectTomlInner {
    // Only its presence matters
    pub build_system: Option<TomlValue>,
    pub dependency_groups: BTreeMap<String, Vec<DependencyGroupEntry>>,
    pub project: Option<Project>,
    pub tool: Option<Tool>,
}

// https://packaging.python.org/en/latest/specifications/pyproject-toml/
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Project {
    pub dependencies: Vec<String>,
    pub dynamic: Vec<String>,
    pub name: Option<String>,
    pub optional_dependencies: BTreeMap<String, Vec<String>>,
    pub version: Option<String>,
}

// https://peps.python.org/pep-0735/
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DependencyGroupEntry {
    Requirement(String),
    // Like `{ include-group = "test" }`. ADR-0008.4
    Other(TomlValue),
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Tool {
    pub poetry: Option<ToolPoetry>,
    pub uv: Option<ToolUv>,
}

// https://python-poetry.org/docs/pyproject/
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolPoetry {
    pub dependencies: BTreeMap<String, PoetryDependency>,
    // Before Poetry 1.2
    pub dev_dependencies: BTreeMap<String, PoetryDependency>,
    pub group: BTreeMap<String, ToolPoetryGroup>,
    // Poetry 1.x, before `[project]`
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolPoetryGroup {
    pub dependencies: BTreeMap<String, PoetryDependency>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PoetryDependency {
    Version(String),
    Table(PoetryDependencyOptions),
    // Constraints that depend on the Python version or platform
    Multiple(Vec<PoetryDependencyOptions>),
    // ADR-0008.4
    Unknown(TomlValue),
}

impl PoetryDependency {
    pub fn get_paths(&self) -> Vec<&str> {
        match self {
            Self::Table(options) => options.path.as_deref().into_iter().collect(),
            Self::Multiple(list) => list
                .iter()
                .filter_map(|options| options.path.as_deref())
                .collect(),
            Self::Version(_) | Self::Unknown(_) => vec![],
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PoetryDependencyOptions {
    pub path: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolUv {
    pub dev_dependencies: Vec<String>,
    pub sources: BTreeMap<String, ToolUvSource>,
    pub workspace: Option<ToolUvWorkspace>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ToolUvSource {
    Single(ToolUvSourceOptions),
    Multiple(Vec<ToolUvSourceOptions>),
    // ADR-0008.4
    Unknown(TomlValue),
}

impl ToolUvSource {
    fn get_options(&self) -> Vec<&ToolUvSourceOptions> {
        match self {
            Self::Single(source) => vec![source],
            Self::Multiple(sources) => sources.iter().collect(),
            Self::Unknown(_) => vec![],
        }
    }

    pub fn get_paths(&self) -> Vec<&str> {
        self.get_options()
            .into_iter()
            .filter_map(|source| source.path.as_deref())
            .collect()
    }

    pub fn is_workspace(&self) -> bool {
        self.get_options()
            .into_iter()
            .any(|source| matches!(source.workspace, Some(ToolUvWorkspaceSource::Boolean(true))))
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ToolUvSourceOptions {
    pub path: Option<String>,
    pub workspace: Option<ToolUvWorkspaceSource>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ToolUvWorkspaceSource {
    Boolean(bool),
    // ADR-0008.4
    Unknown(TomlValue),
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolUvWorkspace {
    pub members: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_pdk_api::VirtualPath;
    use starbase_utils::toml;

    fn parse(content: &str) -> PyProjectToml {
        PyProjectToml {
            data: toml::parse(content).unwrap(),
            dirty: vec![],
            path: VirtualPath::new("/workspace/pyproject.toml"),
        }
    }

    #[test]
    fn skips_requirements_that_cant_be_parsed() {
        let manifest = parse(
            r#"
[project]
name = "a"
dependencies = ["six", "not a requirement!", "idna>=3"]

[project.optional-dependencies]
extra = ["===", "requests"]
"#,
        );

        let names = |reqs: Vec<Requirement>| {
            reqs.into_iter()
                .map(|req| req.name.to_string())
                .collect::<Vec<_>>()
        };

        assert_eq!(names(manifest.get_dependencies()), ["six", "idna"]);
        assert_eq!(names(manifest.get_optional_dependencies()), ["requests"]);
    }

    #[test]
    fn loads_without_a_name() {
        let manifest = parse("[project]\ndependencies = [\"six\"]\n");

        assert_eq!(manifest.project.as_ref().unwrap().name, None);
        assert_eq!(manifest.get_dependencies().len(), 1);
    }

    #[test]
    fn ignores_fields_it_doesnt_read() {
        let manifest = parse(
            r#"
[project]
name = "a"
readme = 123
requires-python = ">=3.8.*"
license = ["unknown"]
authors = "nobody"
"#,
        );

        assert_eq!(
            manifest.project.as_ref().unwrap().name.as_deref(),
            Some("a")
        );
    }

    #[test]
    fn reads_the_poetry_name() {
        let manifest = parse("[tool.poetry]\nname = \"legacy\"\nversion = \"1.0.0\"\n");

        assert_eq!(
            manifest.get_tool_poetry().unwrap().name.as_deref(),
            Some("legacy")
        );
    }

    #[test]
    fn reads_dependency_groups_and_skips_includes() {
        let manifest = parse(
            r#"
[dependency-groups]
dev = ["pytest", { include-group = "lint" }]
lint = ["ruff"]
"#,
        );

        let mut names = manifest
            .get_dependency_groups()
            .into_iter()
            .map(|req| req.name.to_string())
            .collect::<Vec<_>>();
        names.sort();

        assert_eq!(names, ["pytest", "ruff"]);
    }

    #[test]
    fn parses_unknown_source_and_dependency_shapes() {
        let manifest = parse(
            r#"
[tool.uv.sources]
a = { workspace = true }
b = [{ path = "../b" }, { index = "pypi" }]
c = "unknown"
d = { workspace = "yes" }

[tool.poetry.dependencies]
python = "^3.12"
e = { path = "../e", develop = true }
f = [{ version = "1", python = "<3.9" }, { path = "../f" }]
g = 1
"#,
        );

        let uv = manifest.get_tool_uv().unwrap();

        assert!(uv.sources["a"].is_workspace());
        assert_eq!(uv.sources["b"].get_paths(), ["../b"]);
        assert!(matches!(uv.sources["c"], ToolUvSource::Unknown(_)));
        assert!(!uv.sources["d"].is_workspace());

        let poetry = manifest.get_tool_poetry().unwrap();

        assert!(poetry.dependencies["python"].get_paths().is_empty());
        assert_eq!(poetry.dependencies["e"].get_paths(), ["../e"]);
        assert_eq!(poetry.dependencies["f"].get_paths(), ["../f"]);
        assert!(matches!(
            poetry.dependencies["g"],
            PoetryDependency::Unknown(_)
        ));
    }

    #[test]
    fn detects_build_system_and_dynamic_dependencies() {
        let manifest = parse(
            r#"
[build-system]
requires = ["hatchling"]

[project]
name = "a"
dynamic = ["dependencies"]
"#,
        );

        assert!(manifest.has_build_system());
        assert!(manifest.has_dynamic_dependencies());
        assert!(!parse("[project]\nname = \"a\"\n").has_build_system());
    }

    #[test]
    fn normalizes_paths_lexically() {
        assert_eq!(
            normalize_path(Path::new("/workspace/apps/a/../../libs/./b")),
            PathBuf::from("/workspace/libs/b")
        );
    }
}
