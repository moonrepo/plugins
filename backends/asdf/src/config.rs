#![allow(dead_code)]

use extism_pdk::*;
use proto_pdk::*;
use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

const ASDF_PLUGINS_URL: &str =
    "https://raw.githubusercontent.com/asdf-vm/asdf-plugins/refs/heads/master/plugins";

/// https://asdf-vm.com/manage/plugins.html
#[derive(Debug, Default, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct AsdfPluginConfig {
    pub asdf_shortname: Option<String>,
    pub asdf_repository: Option<String>,
    pub executable_name: Option<String>,
}

impl AsdfPluginConfig {
    pub fn get_exe_name(&self) -> AnyResult<String> {
        match &self.executable_name {
            Some(name) => Ok(name.into()),
            None => self.get_id(),
        }
    }

    pub fn get_id(&self) -> AnyResult<String> {
        match &self.asdf_shortname {
            Some(name) => Ok(name.into()),
            None => get_plugin_id(),
        }
    }

    pub fn get_backend_id(&self) -> AnyResult<String> {
        Ok(format!("asdf-{}", self.get_id()?))
    }

    pub fn get_backend_path(&self) -> AnyResult<PathBuf> {
        let backend_id = self.get_backend_id()?;

        Ok(PathBuf::from(format!("/proto/backends/{backend_id}")))
    }

    pub fn get_script_path(&self, script: &str) -> AnyResult<PathBuf> {
        self.get_backend_path()
            .map(|path| path.join("bin").join(script))
    }

    pub fn get_repo_url(&self) -> AnyResult<String> {
        if let Some(repo_url) = &self.asdf_repository {
            return Ok(repo_url.into());
        }

        let id = self.get_id()?;
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
