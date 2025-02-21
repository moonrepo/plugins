use crate::config::AsdfPluginConfig;
use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;
struct Repo {
    url: String,
}

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn from_virtual_path(path: String) -> String;
    fn to_virtual_path(path: String) -> String;
    fn host_log(input: Json<HostLogInput>);
}

const ASDF_PLUGINS_URL: &str = "https://raw.githubusercontent.com/asdf-vm/asdf-plugins/refs/heads/master/plugins";

/// Returns whether the user has opted to use the GitHub repository instead of solely using the asdf short-name
fn is_asdf_repo(config: &AsdfPluginConfig) -> bool {
    config.asdf_repository.is_some()
}

fn get_id(config: Option<AsdfPluginConfig>) -> FnResult<String> {
    let config = config.unwrap_or(get_tool_config::<AsdfPluginConfig>()?);
    Ok(config.asdf_shortname.unwrap_or(get_plugin_id()?))
}

fn get_executable_name() -> FnResult<String> {
    let config = get_tool_config::<AsdfPluginConfig>()?;
    Ok(config.executable_name.clone().unwrap_or(get_id(Some(config))?))
}

fn get_repo() -> FnResult<Repo> {
    let config = get_tool_config::<AsdfPluginConfig>()?;
    if is_asdf_repo(&config) {
        let repo_url = config.asdf_repository.unwrap().trim().to_string();

        return Ok(Repo {
            url: repo_url.to_string(),
        });
    }

    let id = get_id(None)?;
    let filepath = format!("{ASDF_PLUGINS_URL}/{id}");
    let repo_response = send_request!(&filepath);
    let mut repo_url = match repo_response.status {
        200 => Ok::<String, Error>(repo_response.text()?),
        404 => Err(PluginError::Message(format!("URL not found: {filepath}")).into()),
        _ => Err(PluginError::Message(format!("Failed to fetch URL: {filepath}")).into()),
    }?;
    repo_url = repo_url.replace(" ", "");
    let repo_url = repo_url.split("=").last();
    let Some(repo_url) = repo_url else {
        return Err(PluginError::Message(String::from("Repository not found in downloaded file"))
        .into());
    };

    let repo_url = repo_url.split(".git").next();
    let Some(repo_url) = repo_url else {
        return Err(PluginError::Message(String::from("Repository not found in downloaded file"))
        .into());
    };

    Ok(Repo {
        url: repo_url.to_string(),
    })
}

fn get_versions(_: VersionSpec) -> FnResult<Vec<String>> {
    let script_path = get_backend_path()?.join("bin").join("list-all");
    if !script_path.exists() {
        return Err(PluginError::Message("list-all script not found, is the ASDF repository valid?".to_string()).into());
    }

    let script_path = script_path.real_path().unwrap().into_os_string().into_string().unwrap();
    let versions = exec_command!("bash", [script_path]).stdout;
    let versions: Vec<String> = versions.split_whitespace().map(str::to_owned).collect();
    Ok(versions)
}

fn get_backend_id() -> FnResult<String> {
    let id = get_id(None)?;
    Ok(format!("asdf-{id}"))
}

fn get_backend_path() -> FnResult<VirtualPath> {
    let backend_id = get_backend_id()?;
    let virtual_path = virtual_path!(format!("/proto/backends/{backend_id}"));
    Ok(virtual_path)
}

fn exec_script(script_path: VirtualPath) -> FnResult<()> {
    let script_path = script_path.real_path().unwrap().into_os_string().into_string().unwrap();
    let result = exec_command!("bash", [&script_path]);
    if result.exit_code != 0 {
        return Err(PluginError::Message(format!("Failed to execute script ({script_path}): {}", result.stderr)).into());
    }

    Ok(())
}

fn set_env_var(name: &str, value: &str) -> FnResult<()> {
    if let Some(var) = get_host_env_var(name)? {
        host_log!("Skipped setting environment variable '{name}' to '{value}', because it's already set to '{var}'");
    } else {
        set_host_env_var(name, value)?;
    }

    Ok(())
}

fn cpu_cores() -> FnResult<String> {
     let cores = if get_host_environment()?.os.is_mac() {
        exec_command!("sysctl -n hw.physicalcpu").stdout
    } else {
        exec_command!("nproc").stdout
    };

    Ok(cores)
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![
            ".tool-versions".into(),
        ],
        ignore: vec![]
    }))
}

#[plugin_fn]
pub fn parse_version_file(Json(input): Json<ParseVersionFileInput>) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut final_version = None;
    
    if input.file != ".tool-versions" {
        return Err(PluginError::Message("Invalid version file".to_string()).into());
    }

    for line in input.content.lines() {
        let mut parsed_line = String::new();
        for char in line.chars() {
            if char == '#' {
                break;
            }
            parsed_line.push(char);
        }

        let (tool, version) = parsed_line.split_once(' ').unwrap_or((&parsed_line, ""));
        if tool == get_id(None)? && !version.is_empty() {
            final_version = Some(UnresolvedVersionSpec::parse(version)?);
            break;
        }
    }
    
    Ok(Json(ParseVersionFileOutput { version: final_version }))
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: input.id,
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        default_install_strategy: InstallStrategy::BuildFromSource,
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
		config_schema: Some(schematic::SchemaBuilder::generate::<AsdfPluginConfig>()),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn register_backend(Json(_): Json<RegisterBackendInput>) -> FnResult<Json<RegisterBackendOutput>> { 
    if get_host_environment()?.os.is_windows() {
        return Err(PluginError::UnsupportedWindowsBuild.into());
    }

    Ok(Json(RegisterBackendOutput {
        backend_id: get_backend_id()?,
        source: Some(SourceLocation::Git(GitSource {
            url: String::from(get_repo()?.url),
            ..GitSource::default()
        })),
        ..RegisterBackendOutput::default()
    }))
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let install_download_path = real_path!(buf, input.context.tool_dir).into_os_string().into_string().unwrap();
    // Create the download/install path if it doesn't already exist
    if !virtual_path!(&install_download_path).exists() {
        exec_command!("mkdir", ["-p", &install_download_path]);
    }

    // Set asdf environment variables
    set_env_var("ASDF_INSTALL_TYPE", "version")?;
    set_env_var("ASDF_INSTALL_VERSION", input.context.version.to_string().as_str())?;
    set_env_var("ASDF_INSTALL_PATH", &install_download_path)?;
    set_env_var("ASDF_DOWNLOAD_PATH", &install_download_path)?;
    set_env_var("ASDF_CONCURRENCY", cpu_cores()?.as_str())?;

    let download_script_path = get_backend_path()?.join("bin").join("download");
    let install_script_path = get_backend_path()?.join("bin").join("install");
    // In older versions of asdf there may not be a 'download' script,
    // instead both download and install were done in the 'install' script.
    // However, in newer versions, there's two separate 'download' and 'install' scripts.
    if download_script_path.exists() {
        exec_script(download_script_path)?;
    }
    exec_script(install_script_path)?;

    Ok(Json(NativeInstallOutput {
        // TODO: Figure out if returning an Err (FnResult) is different than
        // setting this to false along with the 'error' field
        installed: true,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let exe = get_executable_name()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            exe.clone(),
            ExecutableConfig::new_primary(
                format!("bin/{exe}")
            )
        )]),
        exes_dir: Some("bin".into()),
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
/// Loads all versions, if the version is invalid, skip it. Expects versions to be ordered in descending order.
pub fn load_versions(Json(input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let Ok(mut versions) = get_versions(input.context.version) else {
        return Err(PluginError::Message("Failed to find any version".to_string()).into())
    };
     // Remove the last element, which is the latest version
    let last_version = versions.pop().unwrap();
    let version = UnresolvedVersionSpec::parse(&last_version);

    match version {
        Ok(version) => {
            output.latest = Some(version);
            output.versions.push(VersionSpec::parse(last_version)?);
        },
        _ => return Err(PluginError::Message("Failed to find any version".to_string()).into())
    }

    for version in versions.iter() {
        let version = VersionSpec::parse(version);
        match version {
            Ok(version) => output.versions.push(version),
            _ => continue
        };
    }

    Ok(Json(output))
}