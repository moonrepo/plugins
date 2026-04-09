use proto_pdk::*;

const DEFAULT_BUCKET: &str = "ScoopInstaller/Main";
const DEFAULT_BRANCH: &str = "master";

/// Configuration for the Scoop backend plugin.
/// https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests
#[derive(Debug, Default, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct ScoopToolConfig {
    /// The GitHub repository for the scoop bucket. Defaults to "ScoopInstaller/Main".
    pub bucket: Option<String>,

    /// The branch of the bucket repository. Defaults to "master".
    pub bucket_branch: Option<String>,

    /// Override the manifest filename (without .json extension).
    /// Defaults to the tool ID.
    pub manifest_name: Option<String>,
}

impl ScoopToolConfig {
    pub fn get_bucket(&self) -> &str {
        self.bucket.as_deref().unwrap_or(DEFAULT_BUCKET)
    }

    pub fn get_branch(&self) -> &str {
        self.bucket_branch.as_deref().unwrap_or(DEFAULT_BRANCH)
    }

    pub fn get_manifest_name(&self) -> AnyResult<String> {
        match &self.manifest_name {
            Some(name) => Ok(name.clone()),
            None => Ok(get_plugin_id()?.to_string()),
        }
    }

    pub fn get_manifest_url(&self) -> AnyResult<String> {
        let bucket = self.get_bucket();
        let branch = self.get_branch();
        let name = self.get_manifest_name()?;

        Ok(format!(
            "https://raw.githubusercontent.com/{bucket}/refs/heads/{branch}/bucket/{name}.json"
        ))
    }
}
