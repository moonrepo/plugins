use crate::config::AsdfPluginConfig;
use extism_pdk::json::Value;
use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;
use std::fs;

struct Repo {
    url: String,
    default_branch: String,
}

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn from_virtual_path(path: String) -> String;
    fn to_virtual_path(path: String) -> String;
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
    fn get_default_branch(repo: &str) -> FnResult<String> {
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
                return Ok(default_branch.into());
            },
            _ => Err(PluginError::Message("Failed to fetch repository's default branch".to_string()).into())
        }
    }

    let config = get_tool_config::<AsdfPluginConfig>()?;
    if is_asdf_repo(&config) {
        let repo_url = config.asdf_repository.unwrap().trim().to_string();

        return Ok(Repo {
            url: repo_url.to_string(),
            default_branch: get_default_branch(&repo_url)?
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
        default_branch: get_default_branch(&repo_url)?
    })
}


// Workaround for a bug when using fs::remove_dir_all in this environment
fn remove_dir_recursive(path: &VirtualPath) -> FnResult<()> {
    let path = path.real_path().unwrap().into_os_string().into_string().unwrap();
    if exec_command!("rm", ["-rf", &path]).exit_code != 0 {
        return Err(PluginError::Message("Failed to remove directory".to_string()).into());
    }
    Ok(())
}

fn clone_repo() -> FnResult<VirtualPath> {
    let repo = get_repo()?;
    let repo_dir = virtual_path!(format!("/proto/temp/{}/repo", get_id(None)?));
    // Remove the previous repo directory if it exists
    remove_dir_recursive(&repo_dir)?;
    fs::create_dir_all(&repo_dir.parent().unwrap())?;

    if exec_command!("git", ["clone", "--depth=1", &repo.url, &repo_dir.real_path().unwrap().into_os_string().into_string().unwrap()]).exit_code != 0 {
        return Err(PluginError::Message("Failed to clone repository".to_string()).into());
    }

    Ok(repo_dir)
}

fn get_versions() -> FnResult<Vec<String>> {
    let script_path = clone_repo()?;
    let script_path = script_path.join("bin").join("list-all").real_path().unwrap().into_os_string().into_string().unwrap();

    let versions = exec_command!("bash", [script_path]).stdout;
    let versions: Vec<String> = versions.split_whitespace().map(str::to_owned).collect();
    Ok(versions)
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {    
    Ok(Json(ToolMetadataOutput {
        name: input.id,
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 45, 2)),
        default_install_strategy: InstallStrategy::BuildFromSource,
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
		config_schema: Some(schematic::SchemaBuilder::generate::<AsdfPluginConfig>()),
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn build_instructions(
    Json(input): Json<BuildInstructionsInput>,
) -> FnResult<Json<BuildInstructionsOutput>> {
    let env = get_host_environment()?;    

    if env.os.is_windows() {
        return Err(PluginError::UnsupportedWindowsBuild.into());
    }
    
    let repo = get_repo()?;

    let git = GitSource {
        url: repo.url.clone(),
        // Use default branch
        reference: None,
        ..Default::default()
    };

    let mut instructions = Vec::new();

    let version = input.context.version;

    // Set asdf environment variables
    let install_download_path = &input.context.tool_dir.real_path().unwrap().into_os_string().into_string().unwrap();
    let cores = if env.os.is_mac() {
        exec_command!("sysctl -n hw.physicalcpu").stdout
    } else {
        exec_command!("nproc").stdout
    };
    instructions.append(&mut vec![
        BuildInstruction::SetEnvVar("ASDF_INSTALL_TYPE".into(), "version".into()),
        BuildInstruction::SetEnvVar("ASDF_INSTALL_VERSION".into(), version.clone().into()),
        BuildInstruction::SetEnvVar("ASDF_INSTALL_PATH".into(), install_download_path.clone()),
        BuildInstruction::SetEnvVar("ASDF_DOWNLOAD_PATH".into(), install_download_path.clone()),
        BuildInstruction::SetEnvVar("ASDF_CONCURRENCY".into(), cores),
    ]);

    let id = get_id(None)?;

    // In older versions of asdf there may not be a 'download' script,
    // instead both download and install were done in the 'install' script.
    // However, in newer versions, there's two separate 'download' and 'install' scripts.
    if send_request!(format!("{}/blob/{}/bin/download", repo.url, repo.default_branch)).status == 200 {
            let download_id = String::from(format!("{id}-{version}-download"));
            instructions.push(BuildInstruction::InstallBuilder(Box::new(BuilderInstruction {
                id: download_id.clone(),
                exe: "bin/download".into(),
                git: git.clone()
            })));
            
            instructions.push(
                BuildInstruction::RunCommand(Box::new(CommandInstruction::with_builder(
                    &download_id,
                    [""],
                ))),
            )
        }

    let install_id = String::from(format!("{id}-{version}-install"));
    instructions.push(
        BuildInstruction::InstallBuilder(Box::new(BuilderInstruction {
            id: install_id.clone(),
            exe: "bin/install".into(),
            git,
        }))
    );
    instructions.push(
        BuildInstruction::RunCommand(Box::new(CommandInstruction::with_builder(
            &install_id,
            [""],
        ))),
    );

    let output = BuildInstructionsOutput {
        instructions,
        ..Default::default()
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let exe = get_executable_name()?;

    let install_path = input.context.tool_dir.real_path().unwrap().into_os_string().into_string().unwrap();
    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            exe.clone(),
            ExecutableConfig::new_primary(
                format!("{install_path}/bin/{exe}")
            )
        )]),
        exes_dir: input.context.tool_dir.join("bin").real_path(),
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
/// Loads all versions, if the version is invalid, skip it. Expects versions to be ordered in descending order.
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();

    let Ok(mut versions) = get_versions() else {
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