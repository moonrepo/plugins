use crate::config::AsdfPluginConfig;
use extism_pdk::*;
use proto_pdk::*;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn get_env_var(key: &str) -> String;
    fn to_virtual_path(input: String) -> String;
    fn from_virtual_path(path: String) -> String;
    fn host_log(input: Json<HostLogInput>);
}

#[derive(Debug, Serialize)]
pub struct AsdfPlugin {
    asdf_home: PathBuf,
    config: Option<AsdfPluginConfig>,
}

impl AsdfPlugin {
    fn new() -> Self {
        // Try ASDF_DATA_DIR first
        let asdf_home = if let Ok(data_dir) = unsafe { get_env_var("ASDF_DATA_DIR") } {
            PathBuf::from(data_dir)
        } else {
            // Then try HOME/.asdf
            if let Ok(home) = unsafe { get_env_var("HOME") } {
                let home_asdf = PathBuf::from(home).join(".asdf");
                if home_asdf.exists() {
                    home_asdf
                } else {
                    // Finally try ASDF_DIR
                    if let Ok(asdf_dir) = unsafe { get_env_var("ASDF_DIR") } {
                        PathBuf::from(asdf_dir)
                    } else {
                        home_asdf // Default to HOME/.asdf if nothing else works
                    }
                }
            } else {
                PathBuf::from("/.asdf") // Fallback if no home directory
            }
        };

        Self {
            asdf_home,
            config: None,
        }
    }

    fn get_config_file(&self) -> PathBuf {
        // Try ASDF_CONFIG_FILE first
        if let Ok(config_file) = unsafe { get_env_var("ASDF_CONFIG_FILE") } {
            PathBuf::from(config_file)
        } else {
            // Default to HOME/.asdfrc
            if let Ok(home) = unsafe { get_env_var("HOME") } {
                PathBuf::from(home).join(".asdfrc")
            } else {
                PathBuf::from("/.asdfrc")
            }
        }
    }

    fn create_plugin_scripts(&self, plugin_dir: &Path) -> Result<(), Error> {
        let bin_dir = plugin_dir.join("bin");
        
        // Create required scripts with more comprehensive templates
        let scripts = [
            // Required scripts
            ("list-all", r#"#!/usr/bin/env bash
set -euo pipefail

# Fetch all available versions
# This is a basic implementation. Plugin should customize this.
# Some plugins might need to:
# - Parse HTML pages
# - Use API endpoints
# - Check multiple sources
git ls-remote --tags origin | grep -o 'refs/tags/.*' | cut -d/ -f3- | grep -v '\^{}' | sort -V"#),

            ("download", r#"#!/usr/bin/env bash
set -euo pipefail

# Required environment variables
if [ -z "${ASDF_DOWNLOAD_PATH:-}" ]; then
    echo "ASDF_DOWNLOAD_PATH is required" >&2
    exit 1
fi

if [ -z "${ASDF_INSTALL_VERSION:-}" ]; then
    echo "ASDF_INSTALL_VERSION is required" >&2
    exit 1
fi

mkdir -p "$ASDF_DOWNLOAD_PATH"

# Plugin should implement download strategy:
# - Direct download with curl/wget
# - Git clone specific tag
# - API-based download
# - Platform-specific binaries
echo "Plugin must implement download strategy" >&2
exit 1"#),

            ("install", r#"#!/usr/bin/env bash
set -euo pipefail

# Required environment variables
if [ -z "${ASDF_INSTALL_PATH:-}" ]; then
    echo "ASDF_INSTALL_PATH is required" >&2
    exit 1
fi

if [ -z "${ASDF_INSTALL_VERSION:-}" ]; then
    echo "ASDF_INSTALL_VERSION is required" >&2
    exit 1
fi

if [ -z "${ASDF_DOWNLOAD_PATH:-}" ]; then
    echo "ASDF_DOWNLOAD_PATH is required" >&2
    exit 1
fi

mkdir -p "$ASDF_INSTALL_PATH"

# Plugin should implement installation:
# - Compilation from source
# - Binary installation
# - Dependencies setup
# - Platform-specific steps
echo "Plugin must implement installation strategy" >&2
exit 1"#),

            // Recommended scripts
            ("latest-stable", r#"#!/usr/bin/env bash
set -euo pipefail

# Get latest stable version
# Plugin should customize this based on:
# - Version naming conventions
# - Release channels
# - Platform support
current_script_path=${BASH_SOURCE[0]}
plugin_dir=$(dirname "$(dirname "$current_script_path")")

# Default implementation uses list-all
"$plugin_dir/bin/list-all" | grep -v '[a-zA-Z]' | tail -n1"#),

            // Optional but commonly needed scripts
            ("list-bin-paths", r#"#!/usr/bin/env bash
set -euo pipefail

# List directories containing executables
# Plugin should customize based on:
# - Tool's directory structure
# - Multiple binary locations
# - Platform-specific paths
echo "bin"  # Default to bin directory"#),

            ("exec-env", r#"#!/usr/bin/env bash
set -euo pipefail

# Setup environment for execution
# Plugin should set:
# - PATH additions
# - Tool-specific env vars
# - Dependencies' env vars"#),

            ("list-legacy-filenames", r#"#!/usr/bin/env bash
set -euo pipefail

# List legacy version files
# Examples:
# .ruby-version
# .node-version
# .python-version
echo ""  # Plugin should customize"#),

            ("parse-legacy-file", r#"#!/usr/bin/env bash
set -euo pipefail

# Parse legacy version file
# Input: $1 (legacy file path)
# Output: version string
if [ -f "$1" ]; then
    cat "$1"
fi"#),
        ];

        for (name, content) in scripts {
            let script_path = bin_dir.join(name);
            fs::write(&script_path, content)?;
            
            // Make script executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&script_path)?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&script_path, perms)?;
            }
        }

        Ok(())
    }

    fn handle_command_error(output: &ExecCommandOutput) -> Result<(), Error> {
        if output.exit_code != 0 {
            Err(Error::msg(output.stderr.clone()))
        } else {
            Ok(())
        }
    }

    fn ensure_plugin_installed(&self, config: &AsdfPluginConfig, tool: &str) -> Result<(), Error> {
        let plugin_name = config.asdf_plugin.as_deref().unwrap_or(tool);
        let plugins_dir = self.asdf_home.join("plugins");

        if !plugins_dir.exists() {
            fs::create_dir_all(&plugins_dir)?;
        }

        let plugin_dir = plugins_dir.join(plugin_name);
        if !plugin_dir.exists() {
            let repo_url = config.asdf_repository.clone()
                .unwrap_or_else(|| format!("https://github.com/asdf-vm/asdf-{}.git", plugin_name));

            let plugin_dir_str = plugin_dir.display().to_string();
            let virtual_path = virtual_path!(buf, plugin_dir);

            let mut env = FxHashMap::default();
            env.insert("ASDF_INSTALL_TYPE".to_string(), "version".to_string());
            env.insert("ASDF_INSTALL_VERSION".to_string(), config.asdf_version.clone().unwrap_or_else(|| "latest".to_string()));
            
            let install_path = self.asdf_home.join("installs").join(plugin_name).join(config.asdf_version.clone().unwrap_or_else(|| "latest".to_string()));
            let download_path = self.asdf_home.join("downloads").join(plugin_name).join(config.asdf_version.clone().unwrap_or_else(|| "latest".to_string()));
            
            env.insert("ASDF_INSTALL_PATH".to_string(), install_path.display().to_string());
            env.insert("ASDF_DOWNLOAD_PATH".to_string(), download_path.display().to_string());
            env.insert("ASDF_PLUGIN_PATH".to_string(), plugin_dir.display().to_string());
            
            if let Some(repo) = &config.asdf_repository {
                env.insert("ASDF_PLUGIN_SOURCE_URL".to_string(), repo.clone());
            } else {
                env.insert(
                    "ASDF_PLUGIN_SOURCE_URL".to_string(), 
                    format!("https://github.com/asdf-vm/asdf-{}.git", plugin_name)
                );
            }

            env.insert("ASDF_CMD_FILE".to_string(), virtual_path.to_string());
            
            if let Ok(cpus) = std::thread::available_parallelism() {
                env.insert("ASDF_CONCURRENCY".to_string(), cpus.get().to_string());
            }

            let output = exec_command!(
                input,
                ExecCommandInput {
                    command: "git".into(),
                    args: vec!["clone".into(), repo_url, virtual_path.to_string()],
                    env,
                    set_executable: true,
                    ..ExecCommandInput::default()
                }
            );

            if output.exit_code != 0 {
                return Err(Error::msg(output.stderr));
            }

            let bin_dir = plugin_dir.join("bin");
            fs::create_dir_all(&bin_dir)?;
            
            self.create_plugin_scripts(&plugin_dir)?;
        }

        Ok(())
    }

    fn run_plugin_script(&self, plugin_name: &str, script: &str, version: &Version) -> Result<(), Error> {
        let plugin_dir = self.asdf_home.join("plugins").join(plugin_name);
        let script_path = plugin_dir.join("bin").join(script);

        if !script_path.exists() {
            return Err(Error::msg(format!(
                "Plugin script {} not found for {}",
                script, plugin_name
            )));
        }

        let install_path = self.asdf_home.join("installs").join(plugin_name).join(version.to_string());
        let download_path = self.asdf_home.join("downloads").join(plugin_name).join(version.to_string());

        let mut env = FxHashMap::default();
        env.insert("ASDF_INSTALL_TYPE".to_string(), "version".to_string());
        env.insert("ASDF_INSTALL_VERSION".to_string(), version.to_string());
        env.insert("ASDF_INSTALL_PATH".to_string(), install_path.display().to_string());
        env.insert("ASDF_DOWNLOAD_PATH".to_string(), download_path.display().to_string());
        env.insert("ASDF_PLUGIN_PATH".to_string(), plugin_dir.display().to_string());
        
        if let Some(config) = &self.config {
            if let Some(repo) = &config.asdf_repository {
                env.insert("ASDF_PLUGIN_SOURCE_URL".to_string(), repo.clone());
            } else {
                env.insert(
                    "ASDF_PLUGIN_SOURCE_URL".to_string(), 
                    format!("https://github.com/asdf-vm/asdf-{}.git", plugin_name)
                );
            }
        }

        let script_path_str = script_path.display().to_string();
        let script_virtual_path = virtual_path!(buf, script_path);
        env.insert("ASDF_CMD_FILE".to_string(), script_path.display().to_string());

        if let Ok(cpus) = std::thread::available_parallelism() {
            env.insert("ASDF_CONCURRENCY".to_string(), cpus.get().to_string());
        }

        let output = exec_command!(
            input,
            ExecCommandInput {
                command: script_virtual_path.to_string(),
                env,
                set_executable: true,
                ..ExecCommandInput::default()
            }
        );

        if output.exit_code != 0 {
            // let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(Error::msg(output.stderr));
        }

        Ok(())
    }

    fn parse_tool_versions(&self, path: &Path, tool_name: &str) -> Result<Vec<Version>, Error> {
        // If it's a .tool-versions file, parse normally
        if path.file_name().and_then(|f| f.to_str()) == Some(&self.get_tool_versions_filename()) {
            let content = fs::read_to_string(path)?;
            let mut versions = Vec::new();

            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0] == tool_name {
                    for version_str in &parts[1..] {
                        if let Ok(version) = Version::parse(version_str) {
                            versions.push(version);
                        }
                    }
                }
            }

            return Ok(versions);
        }

        // For legacy files, use the plugin's parse-legacy-file script
        let tool_name_string = tool_name.to_string();
        let plugin_name = self.config.as_ref()
            .and_then(|c| c.asdf_plugin.as_ref())
            .unwrap_or(&tool_name_string);

        let script_path = self.asdf_home
            .join("plugins")
            .join(plugin_name)
            .join("bin")
            .join("parse-legacy-file");

        if script_path.exists() {
            let output = exec_command!(
                input,
                ExecCommandInput {
                    command: script_path.display().to_string(),
                    args: vec![path.display().to_string()],
                    ..ExecCommandInput::default()
                }
            );

            if output.exit_code == 0 {
                if let Ok(version) = Version::parse(output.stdout.trim()) {
                    return Ok(vec![version]);
                }
            }
        }

        Ok(Vec::new())
    }

    fn get_tool_versions_filename(&self) -> String {
        unsafe { get_env_var("ASDF_DEFAULT_TOOL_VERSIONS_FILENAME") }
            .unwrap_or_else(|_| ".tool-versions".to_string())
    }

    fn find_tool_versions(&self, dir: &Path) -> Vec<PathBuf> {
        let mut tool_versions_files = Vec::new();
        let mut current = Some(PathBuf::from(dir));
        let filename = self.get_tool_versions_filename();

        while let Some(dir) = current {
            let tool_versions = dir.join(&filename);
            if tool_versions.exists() {
                tool_versions_files.push(tool_versions);
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }

        if let Ok(home) = unsafe { get_env_var("HOME") } {
            let global_tool_versions = PathBuf::from(home).join(&filename);
            if global_tool_versions.exists() {
                tool_versions_files.push(global_tool_versions);
            }
        }

        tool_versions_files
    }

    fn update_plugin(&self, plugin_name: &str) -> Result<(), Error> {
        let plugin_dir = self.asdf_home.join("plugins").join(plugin_name);
        if !plugin_dir.exists() {
            return Ok(());  // Nothing to update
        }

        let plugin_dir_str = plugin_dir.display().to_string();
        let virtual_path = virtual_path!(buf, plugin_dir);

        // Get current ref
        let output = exec_command!(
            input,
            ExecCommandInput {
                command: "git".into(),
                args: vec!["rev-parse".into(), "HEAD".into()],
                working_dir: Some(virtual_path.clone()),
                ..ExecCommandInput::default()
            }
        );

        let prev_ref = if output.exit_code == 0 {
            output.stdout
        } else {
            String::new()
        };

        // Fetch and update
        let output = exec_command!(
            input,
            ExecCommandInput {
                command: "git".into(),
                args: vec!["pull".into(), "origin".into(), "master".into()],
                working_dir: Some(virtual_path.clone()),
                ..ExecCommandInput::default()
            }
        );

        if output.exit_code != 0 {
            return Err(Error::msg(output.stderr));
        }

        // Get updated ref
        let output = exec_command!(
            input,
            ExecCommandInput {
                command: "git".into(),
                args: vec!["rev-parse".into(), "HEAD".into()],
                working_dir: Some(virtual_path.clone()),
                ..ExecCommandInput::default()
            }
        );

        let post_ref = if output.exit_code == 0 {
            output.stdout
        } else {
            String::new()
        };

        // Run post-plugin-update script if it exists
        let script_path = plugin_dir.join("bin").join("post-plugin-update");
        if script_path.exists() {
            let mut env = FxHashMap::default();
            env.insert("ASDF_PLUGIN_PATH".to_string(), virtual_path.to_string());
            env.insert("ASDF_PLUGIN_PREV_REF".to_string(), prev_ref);
            env.insert("ASDF_PLUGIN_POST_REF".to_string(), post_ref);

            let script_path_str = script_path.display().to_string();
            let script_virtual_path: VirtualPath = virtual_path!(buf, script_path);
            env.insert("ASDF_CMD_FILE".to_string(), script_path.display().to_string());

            let output = exec_command!(
                input,
                ExecCommandInput {
                    command: script_virtual_path.to_string(),
                    env,
                    set_executable: true,
                    ..ExecCommandInput::default()
                }
            );

            if output.exit_code != 0 {
                return Err(Error::msg(output.stderr));
            }
        }

        Ok(())
    }
}


#[plugin_fn]
pub fn register_tool(Json(_): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: "asdf".into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 42, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    let plugin = AsdfPlugin::new();
    let config = plugin.config.as_ref().ok_or_else(|| {
        Error::msg("Plugin configuration not available")
    })?;
    
    let plugin_id = get_plugin_id()?.to_string();
    let plugin_name = config.asdf_plugin.as_deref().unwrap_or(&plugin_id);
    
    // Get standard .tool-versions file
    let mut files = vec![plugin.get_tool_versions_filename()];
    
    // Get legacy version files from plugin
    let script_path = plugin.asdf_home
        .join("plugins")
        .join(plugin_name)
        .join("bin")
        .join("list-legacy-filenames");
    
    if script_path.exists() {
        let output = exec_command!(
            input,
            ExecCommandInput {
                command: script_path.display().to_string(),
                ..ExecCommandInput::default()
            }
        );

        if output.exit_code == 0 {
            files.extend(
                output.stdout
                    .split_whitespace()
                    .map(|s| s.to_string())
            );
        }
    }

    Ok(Json(DetectVersionOutput {
        files: files.into_iter().map(Into::into).collect(),
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn download(Json(input): Json<ToolContext>) -> FnResult<Json<()>> {
    let plugin = AsdfPlugin::new();
    let config = plugin.config.as_ref().ok_or_else(|| {
        Error::msg("Plugin configuration not available")
    })?;
    
    let plugin_id = get_plugin_id()?.to_string();
    let plugin_name = config.asdf_plugin.as_deref().unwrap_or(&plugin_id);
    plugin.ensure_plugin_installed(config, plugin_name)?;
    
    let version = Version::parse(&input.version.to_string())
        .map_err(|e| Error::msg(format!("Invalid version: {}", e)))?;
    
    let download_path = plugin.asdf_home
        .join("downloads")
        .join(plugin_name)
        .join(version.to_string());
    fs::create_dir_all(&download_path)
        .map_err(|e| Error::msg(e.to_string()))?;

    plugin.run_plugin_script(plugin_name, "download", &version)?;
    Ok(Json(()))
}

#[plugin_fn]
pub fn install(Json(input): Json<ToolContext>) -> FnResult<Json<()>> {
    let plugin = AsdfPlugin::new();
    let config = plugin.config.as_ref().ok_or_else(|| {
        Error::msg("Plugin configuration not available")
    })?;
    
    let plugin_id = get_plugin_id()?.to_string();
    let plugin_name = config.asdf_plugin.as_deref().unwrap_or(&plugin_id);
    
    let version = Version::parse(&input.version.to_string())
        .map_err(|e| Error::msg(format!("Invalid version: {}", e)))?;
    
    let install_path = plugin.asdf_home
        .join("installs")
        .join(plugin_name)
        .join(version.to_string());
    fs::create_dir_all(&install_path)
        .map_err(|e| Error::msg(e.to_string()))?;

    plugin.run_plugin_script(plugin_name, "install", &version)?;
    Ok(Json(()))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == ".tool-versions" {
        for line in input.content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    version = Some(UnresolvedVersionSpec::parse(parts[1].to_string())?);
                    break;
                }
            }
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(_: Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let plugin = AsdfPlugin::new();
    let config = plugin.config.as_ref().ok_or_else(|| {
        Error::msg("Plugin configuration not available")
    })?;
    
    let plugin_id = get_plugin_id()?.to_string();
    let plugin_name = config.asdf_plugin.as_deref().unwrap_or(&plugin_id);
    plugin.ensure_plugin_installed(config, plugin_name)?;
    
    // Update plugin to get latest versions
    plugin.update_plugin(plugin_name)?;
    
    // Run list-all script to get available versions
    let script_path = plugin.asdf_home
        .join("plugins")
        .join(plugin_name)
        .join("bin")
        .join("list-all");

    let output = exec_command!(
        input,
        ExecCommandInput {
            command: script_path.display().to_string(),
            ..ExecCommandInput::default()
        }
    );

    if let Err(e) = AsdfPlugin::handle_command_error(&output) {
        return Err(e.into());
    }

    let mut versions = Vec::new();
    let mut latest = None;
    let mut aliases = FxHashMap::default();

    // Parse versions from list-all output
    for version_str in output.stdout.split_whitespace() {
        if let Ok(version) = Version::parse(version_str) {
            if let Ok(version_spec) = VersionSpec::parse(&version.to_string()) {
                versions.push(version_spec);
            }
        }
    }

    // Get latest stable version
    let latest_script = plugin.asdf_home
        .join("plugins")
        .join(plugin_name)
        .join("bin")
        .join("latest-stable");

    if latest_script.exists() {
        let output = exec_command!(
            input,
            ExecCommandInput {
                command: latest_script.display().to_string(),
                ..ExecCommandInput::default()
            }
        );

        if output.exit_code == 0 {
            if let Ok(version) = Version::parse(output.stdout.trim()) {
                if let Ok(version_spec) = UnresolvedVersionSpec::parse(&version.to_string()) {
                    latest = Some(version_spec.clone());
                    aliases.insert("latest".to_string(), version_spec);
                }
            }
        }
    }

    Ok(Json(LoadVersionsOutput {
        versions,
        latest,
        aliases,
        canary: None,
    }))
} 