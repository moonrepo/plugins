#![allow(dead_code)]

use extism_pdk::*;
use proto_pdk::*;
use schematic::Schematic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

const ASDF_PLUGINS_URL: &str =
    "https://raw.githubusercontent.com/asdf-vm/asdf-plugins/refs/heads/master/plugins";

/// Configuration for the `asdf` backend plugin.
#[derive(Debug, Default, Deserialize, Schematic, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct AsdfToolConfig {
    /// List of binary names to explicit use when locating executables.
    pub exes: Option<Vec<String>>,

    /// Plugin shortname to use for repository resolution.
    #[serde(alias = "asdf-shortname")]
    pub shortname: Option<String>,

    /// Custom Git repository to resolve from.
    #[serde(alias = "asdf-repository")]
    pub repository: Option<String>,
}

// https://asdf-vm.com/manage/plugins.html
impl AsdfToolConfig {
    pub fn get_shortname(&self) -> AnyResult<Id> {
        match &self.shortname {
            Some(name) => Ok(name.into()),
            None => get_plugin_id(),
        }
    }

    pub fn get_backend_id(&self) -> AnyResult<Id> {
        self.get_shortname()
    }

    pub fn get_backend_path(&self) -> AnyResult<PathBuf> {
        Ok(PathBuf::from(format!(
            "/proto/backends/asdf/{}",
            self.get_backend_id()?
        )))
    }

    pub fn get_script_path(&self, script: &str) -> AnyResult<PathBuf> {
        self.get_backend_path()
            .map(|path| path.join("bin").join(script))
    }

    pub fn get_repo_url(&self) -> AnyResult<String> {
        if let Some(repo_url) = &self.repository {
            return Ok(repo_url.into());
        }

        let id = self.get_shortname()?;
        let filepath = format!("{ASDF_PLUGINS_URL}/{id}");
        let repo_response = send_request!(&filepath);

        let repo_config = match repo_response.status {
            200 => Ok::<String, Error>(repo_response.text()?),
            404 => Err(PluginError::Message(format!("URL not found: {filepath}")).into()),
            _ => Err(PluginError::Message(format!("Failed to fetch URL: {filepath}")).into()),
        }?;

        let Some(repo_url) = repo_config.split("=").last() else {
            return Err(PluginError::Message(String::from(
                "Repository not found in downloaded file!",
            ))
            .into());
        };

        Ok(repo_url.trim().trim_end_matches(".git").into())
    }
}
