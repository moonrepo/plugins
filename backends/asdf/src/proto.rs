use crate::config::AsdfPluginConfig;
use extism_pdk::*;
use proto_pdk::*;
use starbase_utils::fs;
use std::{collections::HashMap, path::PathBuf};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn from_virtual_path(path: String) -> String;
    fn to_virtual_path(path: String) -> String;
    fn host_log(input: Json<HostLogInput>);
}

fn exec_script(virtual_script_path: PathBuf) -> AnyResult<String> {
    let script_path = into_real_path(virtual_script_path)?
        .to_string_lossy()
        .to_string();
    let result = exec_captured("bash", [&script_path])?;

    if result.exit_code != 0 {
        return Err(PluginError::Message(format!(
            "Failed to execute script ({script_path}): {}",
            result.stderr
        ))
        .into());
    }

    Ok(result.stdout)
}

fn set_env_var(name: impl AsRef<str>, value: impl AsRef<str>) -> AnyResult<()> {
    let name = name.as_ref();
    let value = value.as_ref();

    match get_host_env_var(name)? {
        Some(var) => {
            host_log!(
                "Skipped setting environment variable '{name}' to '{value}', because it's already set to '{var}'"
            );
        }
        _ => {
            set_host_env_var(name, value)?;
        }
    }

    Ok(())
}

fn cpu_cores() -> AnyResult<String> {
    let result = if get_host_environment()?.os.is_mac() {
        exec_captured("sysctl", ["-n", "hw.physicalcpu"])?
    } else {
        exec_captured("nproc", Vec::<String>::new())?
    };

    Ok(result.stdout.trim().into())
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: input.id,
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        config_schema: Some(schematic::SchemaBuilder::generate::<AsdfPluginConfig>()),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn register_backend(
    Json(_): Json<RegisterBackendInput>,
) -> FnResult<Json<RegisterBackendOutput>> {
    if get_host_environment()?.os.is_windows() {
        return Err(PluginError::UnsupportedWindowsBuild.into());
    }

    let config = get_tool_config::<AsdfPluginConfig>()?;

    Ok(Json(RegisterBackendOutput {
        backend_id: config.get_backend_id()?,
        source: Some(SourceLocation::Git(GitSource {
            url: config.get_repo_url()?,
            ..GitSource::default()
        })),
        ..RegisterBackendOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".tool-versions".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut final_version = None;

    if input.file != ".tool-versions" {
        return Err(PluginError::Message("Invalid version file".to_string()).into());
    }

    let config = get_tool_config::<AsdfPluginConfig>()?;
    let id = config.get_id()?;

    for line in input.content.lines() {
        let mut parsed_line = String::new();

        // Strip comments
        for char in line.chars() {
            if char == '#' {
                break;
            }
            parsed_line.push(char);
        }

        let (tool, version) = parsed_line.split_once(' ').unwrap_or((&parsed_line, ""));

        if tool == id && !version.is_empty() {
            final_version = Some(UnresolvedVersionSpec::parse(version)?);
            break;
        }
    }

    Ok(Json(ParseVersionFileOutput {
        version: final_version,
    }))
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let install_download_path = into_real_path(input.context.tool_dir.any_path())?
        .to_string_lossy()
        .to_string();

    // Create the download/install path if it doesn't already exist
    fs::create_dir_all(&input.context.tool_dir)?;

    // Set asdf environment variables
    set_env_var("ASDF_INSTALL_TYPE", "version")?;
    set_env_var("ASDF_INSTALL_VERSION", input.context.version.to_string())?;
    set_env_var("ASDF_INSTALL_PATH", &install_download_path)?;
    set_env_var("ASDF_DOWNLOAD_PATH", &install_download_path)?;
    set_env_var("ASDF_CONCURRENCY", cpu_cores()?)?;

    let download_script_path = config.get_backend_path()?.join("bin").join("download");
    let install_script_path = config.get_backend_path()?.join("bin").join("install");

    // In older versions of asdf there may not be a 'download' script,
    // instead both download and install were done in the 'install' script.
    // However, in newer versions, there's two separate 'download' and 'install' scripts.
    if download_script_path.exists() {
        exec_script(download_script_path)?;
    }

    exec_script(install_script_path)?;

    Ok(Json(NativeInstallOutput {
        installed: true,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let exe = config.get_exe_name()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            exe.clone(),
            ExecutableConfig::new_primary(format!("bin/{exe}")),
        )]),
        // TODO verify
        exes_dir: Some("bin".into()),
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let script_path = config.get_backend_path()?.join("bin").join("list-all");

    if !script_path.exists() {
        return Err(PluginError::Message(
            "list-all script not found, is the asdf repository valid?".to_string(),
        )
        .into());
    }

    let mut versions: Vec<String> = exec_script(script_path)?
        .split_whitespace()
        .map(str::to_owned)
        .collect();

    if versions.is_empty() {
        return Err(PluginError::Message("Failed to find any versions!".to_string()).into());
    }

    // Remove the last element, which is the latest version
    let last_version = versions.pop().unwrap();
    let version = UnresolvedVersionSpec::parse(&last_version);

    match version {
        Ok(version) => {
            output.latest = Some(version);
            output.versions.push(VersionSpec::parse(last_version)?);
        }
        _ => return Err(PluginError::Message("Failed to find any version".to_string()).into()),
    }

    for version in versions.iter() {
        match VersionSpec::parse(version) {
            Ok(version) => output.versions.push(version),
            _ => continue,
        };
    }

    Ok(Json(output))
}
