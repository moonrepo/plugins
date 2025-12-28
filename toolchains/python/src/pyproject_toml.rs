// `pyproject.toml`

#[cfg(feature = "wasm")]
use extism_pdk::*;
#[cfg(feature = "wasm")]
use moon_pdk::{HostLogInput, host_log};
use moon_pdk_api::{AnyResult, toml_config};
use pep508_rs::Requirement;
use pyproject_toml::PyProjectToml as BasePyProjectToml;
use serde::{Deserialize, Serialize};
use starbase_utils::toml::{self, TomlValue};

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

toml_config!(PyProjectToml, PyProjectTomlInner);

#[allow(dead_code)]
impl PyProjectToml {
    pub fn save_field(&self, _field: &str, _config: &mut TomlValue) -> AnyResult<()> {
        Ok(())
    }
}

// `pyproject_toml::PyProjectToml` does not implement `Default`,
// so we have this hacky workaround...
#[derive(Deserialize, Serialize)]
pub struct PyProjectTomlInner(BasePyProjectToml);

impl PyProjectTomlInner {
    pub fn new() -> Self {
        Self(toml::parse("[package]\nname = \"\"").unwrap())
    }
}

impl Default for PyProjectTomlInner {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for PyProjectTomlInner {
    type Target = BasePyProjectToml;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PyProjectTomlInner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Workspace support: https://docs.astral.sh/uv/concepts/projects/workspaces/
// Only define fields we need!

toml_config!(PyProjectTomlWithTools, PyProjectTomlWithToolsInner);

impl PyProjectTomlWithTools {
    #[allow(dead_code)]
    pub fn save_field(&self, _field: &str, _config: &mut TomlValue) -> AnyResult<()> {
        Ok(())
    }

    /// Extract package members if the current manifest is a workspace.
    pub fn extract_members(&self) -> AnyResult<Option<Vec<String>>> {
        if let Some(workspace) = self
            .data
            .tool
            .as_ref()
            .and_then(|tool| tool.uv.as_ref())
            .and_then(|uv| uv.workspace.as_ref())
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
                return Ok(Some(members));
            }
        }

        Ok(None)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PyProjectTomlWithToolsInner {
    pub tool: Option<Tool>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Tool {
    pub uv: Option<ToolUv>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolUv {
    pub dev_dependencies: Vec<Requirement>,
    pub workspace: Option<ToolUvWorkspace>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolUvWorkspace {
    pub members: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}
