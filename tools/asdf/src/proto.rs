use crate::config::AsdfPluginConfig;
use extism_pdk::*;
use json::Value;
use proto_pdk::*;
use std::collections::HashMap;
use std::fs;

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

fn get_repo_url() -> FnResult<String> {
    let config = get_tool_config::<AsdfPluginConfig>()?;
    if is_asdf_repo(&config) {
        return Ok(config.asdf_repository.unwrap().trim().to_string());
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

    let repo = repo.split(".git").next();
    let Some(repo) = repo else {
        return Err(PluginError::Message(String::from("Repository not found in downloaded file"))
        .into());
    };

    Ok(repo.into())
}

fn get_versions() -> FnResult<Vec<String>> {
    let id = get_plugin_id()?;

    let script_dir = virtual_path!(format!("/proto/temp/{id}"));
    let virtual_script_path = script_dir.join("list-all");
    let script_path = virtual_script_path.real_path().unwrap();
    let script_path = script_path.to_str().unwrap();
    let repo = get_repo_url()?;
    let default_branch = get_default_branch(&repo)?;
    let script_url = format!(
        "{}/refs/heads/{default_branch}/bin/list-all",
        repo.replace("https://github.com", "https://raw.githubusercontent.com")
    );

    let clean  = || {
        Ok::<i32, Error>(exec_command!("rm", ["-rf", script_dir.real_path().unwrap().to_str().unwrap()]).exit_code)
    };

    clean()?;
    fs::create_dir_all(&script_dir)?;
    fs::write(&virtual_script_path, fetch_text(script_url)?)?;
    let versions = exec_command!("bash", [script_path]).stdout;
    let versions: Vec<String> = versions.split_whitespace().map(str::to_owned).collect();
    clean()?;

    Ok(versions)
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {    
    Ok(Json(ToolMetadataOutput {
        name: input.id,
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 45, 0)),
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

    let repo = get_repo_url()?;
    let download_script = fetch_text(format!("{repo}/bin/download"));

    let git = GitSource {
        url: repo.clone(),
        // Use default branch
        reference: None,
        ..Default::default()
    };

    let mut instructions = Vec::new();

    // Set asdf environment variables
    let install_download_path = input.context.tool_dir.real_path().unwrap().into_os_string().into_string().unwrap();
    let cores = if env.os.is_mac() {
        exec_command!("sysctl -n hw.physicalcpu").stdout
    } else {
        exec_command!("nproc").stdout
    };
    instructions.append(&mut vec![
        BuildInstruction::SetEnvVar("ASDF_INSTALL_TYPE".into(), "version".into()),
        BuildInstruction::SetEnvVar("ASDF_INSTALL_VERSION".into(), input.context.version.into()),
        BuildInstruction::SetEnvVar("ASDF_INSTALL_PATH".into(), install_download_path.clone()),
        BuildInstruction::SetEnvVar("ASDF_DOWNLOAD_PATH".into(), install_download_path.clone()),
        BuildInstruction::SetEnvVar("ASDF_CONCURRENCY".into(), cores)
    ]);

    // In older versions of asdf there may not be a 'download' script,
    // instead both download and install were done in the 'install' script.
    // However, in newer versions, there's two separate 'download' and 'install' scripts.
    match download_script {
        Ok(_) => {
            let download_id = String::from("download");
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
        },
        _ => ()
    };

    let install_id = String::from("install");
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
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let id = match config.asdf_shortname {
        Some(shortname) => shortname,
        None => get_plugin_id()?,
    };

    let install_path = input.context.tool_dir.real_path().unwrap().into_os_string().into_string().unwrap();
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

    let mut versions = get_versions()?;
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