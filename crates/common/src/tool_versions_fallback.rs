//! Toolchains embed tools, and moon doesn't provide proto's `load_tool_versions`
//! host function, so the plugin would fail to load. Instead, load versions like
//! the host function does, without the user's `[tools.*.versions-source]`.

use proto_pdk::{
    AnyResult, ExecCommandInput, LoadToolVersionsInput, LoadToolVersionsOutput, SendRequestInput,
    anyhow, exec, fetch, select_index_versions,
};

/// Load versions from a Git remote or JSON index, within the plugin.
pub fn load_tool_versions(input: LoadToolVersionsInput) -> AnyResult<LoadToolVersionsOutput> {
    match input {
        LoadToolVersionsInput::Git { url, tag_pattern } => {
            // Embedded tools only load Git tags with `load_git_tags`
            if tag_pattern.is_some() {
                return Err(anyhow!("Git tag patterns are not supported in toolchains."));
            }

            let args = [
                "ls-remote",
                "--tags",
                "--sort",
                "version:refname",
                url.as_str(),
            ];
            let output = exec(ExecCommandInput::pipe("git", args).error_on_nonzero(true))?;

            Ok(LoadToolVersionsOutput {
                versions: output
                    .stdout
                    .lines()
                    // Peeled tags point to the commit of an annotated tag
                    .filter(|line| !line.ends_with("^{}"))
                    .filter_map(|line| line.split_once('\t')?.1.strip_prefix("refs/tags/"))
                    .map(|tag| tag.to_owned())
                    .collect(),
                body: None,
            })
        }
        LoadToolVersionsInput::Index {
            url,
            version_key,
            headers,
            include_body,
        } => {
            let body = fetch(SendRequestInput {
                url,
                headers,
                ..Default::default()
            })?
            .text()?;
            let index = serde_json::from_str(&body)?;

            Ok(LoadToolVersionsOutput {
                versions: select_index_versions(
                    &index,
                    version_key.as_deref().unwrap_or("*.version"),
                )?,
                body: include_body.then_some(body),
            })
        }
    }
}

/// Load tags from a Git remote, within the plugin.
pub fn load_git_tags(url: impl AsRef<str>) -> AnyResult<Vec<String>> {
    Ok(load_tool_versions(LoadToolVersionsInput::Git {
        url: url.as_ref().to_owned(),
        tag_pattern: None,
    })?
    .versions)
}
