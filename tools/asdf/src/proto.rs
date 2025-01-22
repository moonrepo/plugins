use crate::config::AsdfPluginConfig;
use extism_pdk::*;
use json::Value;
use proto_pdk::*;
use std::{collections::HashMap, fs};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn get_env_var(key: &str) -> String;
    fn set_env_var(name: String, value: String);
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn from_virtual_path(path: String) -> String;
    fn to_virtual_path(path: String) -> String;
}

const ASDF_PLUGINS_URL: &str = "https://raw.githubusercontent.com/asdf-vm/asdf-plugins/refs/heads/master/plugins";

/// Returns whether the user has opted to use the GitHub repository instead of solely using the asdf short-name
fn is_asdf_repo(config: &AsdfPluginConfig) -> bool {
    config.asdf_repository.is_some()
}

fn get_raw_github_url(repo: &str) -> FnResult<String> {
    let repo_parts: Vec<&str> = repo.split("/").collect();

    // Get the default branch of the repository
    let repo_data: Result<_, Error> = fetch_json::<std::string::String, Value>(format!("https://api.github.com/repos/{}/{}", repo_parts[3], repo_parts[4]));
    match repo_data {
        Ok(repo_data) => {
            let default_branch = match repo_data.get("default_branch") {
                Some(branch) => branch,
                None => &Value::String(String::from("main"))
            };
            let default_branch = default_branch.as_str().unwrap();
            let repo = repo.replace("https://github.com", "https://raw.githubusercontent.com");

            return Ok(format!("{repo}/refs/heads/{default_branch}"));
        },
        _ => Err(PluginError::Message("Failed to fetch repository's default branch".to_string()).into())
    }
}

fn get_script_url(script: &str, git: bool) -> FnResult<String> {
    let config = get_tool_config::<AsdfPluginConfig>()?;
    if is_asdf_repo(&config) {
        if git {
            return Ok(config.asdf_repository.unwrap().trim().to_string());
        }
        return Ok(format!("{}/bin/{script}", get_raw_github_url(config.asdf_repository.unwrap().as_str().split(".git").next().unwrap())?));
    }

    let id = get_plugin_id()?;
    let id = if config.asdf_shortname.is_none() { id.as_str() } else { config.asdf_shortname.as_ref().unwrap() };
    let filepath = format!("{ASDF_PLUGINS_URL}/{id}");

    let repo_response = send_request!(&filepath);
    let mut repo = match repo_response.status {
        200 => Ok::<String, Error>(repo_response.text()?),
        404 => Err(PluginError::Message(format!("URL not found: {filepath}")).into()),
        _ => Err(PluginError::Message(format!("Failed to fetch URL: {filepath}")).into()),
    }?;
    repo = repo.replace(" ", "");
    let repo = repo.split("=").last();
    let Some(repo) = repo else {
        return Err(PluginError::Message(String::from("Repository not found in downloaded file"))
        .into());
    };

    if git {
        return Ok(repo.trim().to_string());
    }

    let repo = repo.split(".git").next();
    let Some(repo) = repo else {
        return Err(PluginError::Message(String::from("Repository not found in downloaded file"))
        .into());
    };

    Ok(format!("{}/bin/{script}", get_raw_github_url(repo)?))
}

/// Returns the real path of the script
fn get_script(script: &str) -> FnResult<String> {
    let id = get_plugin_id()?;

    let virtual_script_path = virtual_path!(format!("/proto/temp/{id}"));
    let script_url = get_script_url(script, true)?;
    let repo_dir = virtual_script_path.join("asdf");
    
    fs::create_dir_all(virtual_script_path.parent().unwrap())?;
    exec_command!("rm", ["-rf", repo_dir.real_path().unwrap().to_str().unwrap()]);
    exec_command!("git", ["clone", "--depth=1", &script_url, repo_dir.real_path().unwrap().to_str().unwrap()]);

    let real_script_path = virtual_script_path.join("asdf").join("bin").real_path().unwrap().into_os_string().into_string().unwrap();
    Ok(format!("{real_script_path}/{script}"))
}

/// Set the environment variables to be used by asdf scripts
fn set_asdf_env_vars(version: &str) -> FnResult<()> {
    host_env!("ASDF_INSTALL_TYPE", "version");
    host_env!("ASDF_INSTALL_VERSION", version);

    let asdf_bin = virtual_path!("/proto/bin/asdf");
    host_env!("ASDF_INSTALL_PATH", asdf_bin.real_path().unwrap().to_str().unwrap());
    asdf_bin.real_path().unwrap().to_str().unwrap();
    host_env!("ASDF_DOWNLOAD_PATH", asdf_bin.real_path().unwrap().to_str().unwrap());

    let cores = exec_command!("nproc").stdout;
    host_env!("ASDF_CONCURRENCY", cores);

    Ok(())
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {    
    Ok(Json(ToolMetadataOutput {
        name: input.id,
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 44, 7)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
		config_schema: Some(schematic::SchemaBuilder::generate::<AsdfPluginConfig>()),
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn download_prebuilt(Json(_): Json<DownloadPrebuiltInput>) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
	let id = get_plugin_id()?;

    check_supported_os_and_arch(
        &id,
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64, HostArch::Arm, HostArch::Powerpc64, HostArch::S390x],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
        ],
    )?;
    
    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(String::from("/asdf/temp")),
        download_url: get_script_url("install", false)?,
        download_name: Some(String::from("list-all")),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn unpack_archive(Json(input): Json<UnpackArchiveInput>) -> FnResult<()> {
    set_asdf_env_vars(input.context.version.to_string().as_str())?;

    let download_script = get_script("download");
    // In older versions of asdf there may not be a 'download' script,
    // instead both download and install were done in the 'install' script.
    // However, in newer versions, there's two separate 'download' and 'install' scripts.
    match download_script {
        // Newer versions
        Ok(download_script) => {
            exec_command!("bash", [download_script]).stdout;

            // Install script was already downloaded by the `download_prebuilt` function
            let install_script = get_script("install")?;
            let install_script = install_script.as_str();
            exec_command!("bash", [install_script]);
        }
        // Older versions
        Err(_) => {
            let download_and_install_script = get_script("install")?;
            exec_command!("bash", [download_and_install_script]);
        }
    }

    Ok(())
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    set_asdf_env_vars(input.context.version.to_string().as_str())?;

    let config = get_tool_config::<AsdfPluginConfig>()?;
    let id = match config.asdf_shortname {
        Some(shortname) => shortname,
        None => get_plugin_id()?,
    };
    let install_path = host_env!("ASDF_INSTALL_PATH");
    let Some(install_path) = install_path else {
        return Err(PluginError::Message("ASDF_DOWNLOAD_PATH environment variable not found".to_string()).into());
    };

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                id.clone(),
                ExecutableConfig::new_primary(
                    format!("{install_path}/bin/{id}")
                )
            ),
        ]),
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
/// Loads all versions, if the version is invalid, skip it. Expects versions to be ordered in descending order.
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();

    let versions = exec_command!("bash", [get_script("list-all")?]).stdout;
    let mut versions: Vec<&str> = versions.split_whitespace().map(AsRef::as_ref).collect();
    
     // Remove the last element, which is the latest version
    let last_version = versions.pop().unwrap();
    let version = UnresolvedVersionSpec::parse(last_version);
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