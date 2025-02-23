use crate::config::AsdfPluginConfig;
use extism_pdk::*;
use proto_pdk::*;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
    fn from_virtual_path(path: String) -> String;
    fn to_virtual_path(path: String) -> String;
    fn host_log(input: Json<HostLogInput>);
}

fn exec_script(
    virtual_script_path: PathBuf,
    base_args: Vec<String>,
    env: FxHashMap<String, String>,
) -> AnyResult<String> {
    if !virtual_script_path.exists() {
        return Err(PluginError::Message(format!(
            "{} script not found, is the asdf repository valid?",
            fs::file_name(&virtual_script_path)
        ))
        .into());
    }

    let script_path = into_real_path(virtual_script_path)?
        .to_string_lossy()
        .to_string();

    let mut args = vec![script_path.clone()];
    args.extend(base_args);

    let result = exec(ExecCommandInput {
        command: "bash".into(),
        args,
        env,
        set_executable: true,
        // working_dir, // TODO
        ..Default::default()
    })?;

    if result.exit_code != 0 {
        return Err(PluginError::Message(format!(
            "Failed to execute script ({script_path}): {}",
            result.stderr
        ))
        .into());
    }

    Ok(result.stdout)
}

fn exec_bare_script(virtual_script_path: PathBuf) -> AnyResult<String> {
    exec_script(virtual_script_path, vec![], FxHashMap::default())
}

fn get_env_vars(context: &ToolContext) -> AnyResult<FxHashMap<String, String>> {
    let mut vars = FxHashMap::default();
    vars.insert("ASDF_INSTALL_TYPE".into(), "version".into());
    vars.insert("ASDF_INSTALL_VERSION".into(), context.version.to_string());
    vars.insert(
        "ASDF_INSTALL_PATH".into(),
        context
            .tool_dir
            .real_path()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    );
    vars.insert(
        "ASDF_DOWNLOAD_PATH".into(),
        context
            .temp_dir
            .real_path()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    );
    vars.insert("ASDF_CONCURRENCY".into(), cpu_cores()?);
    Ok(vars)
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
        name: format!("asdf:{}", input.id),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        config_schema: Some(schematic::SchemaBuilder::generate::<AsdfPluginConfig>()),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn register_backend(
    Json(input): Json<RegisterBackendInput>,
) -> FnResult<Json<RegisterBackendOutput>> {
    if get_host_environment()?.os.is_windows() {
        return Err(PluginError::UnsupportedOS {
            tool: input.id,
            os: "windows".into(),
        }
        .into());
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
pub fn detect_version_files(
    Json(input): Json<DetectVersionInput>,
) -> FnResult<Json<DetectVersionOutput>> {
    let mut output = DetectVersionOutput::default();
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let script_path = config.get_script_path("list-legacy-filenames")?;

    output.files = vec![".tool-versions".into()];

    // https://asdf-vm.com/plugins/create.html#bin-list-legacy-filenames
    if script_path.exists() {
        let data = exec_script(script_path, vec![], get_env_vars(&input.context)?)?;

        for file in data.trim().split_whitespace() {
            output.files.push(file.to_owned());
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut output = ParseVersionFileOutput::default();
    let config = get_tool_config::<AsdfPluginConfig>()?;

    if input.file == ".tool-versions" {
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
                output.version = Some(UnresolvedVersionSpec::parse(version)?);
                break;
            }
        }
    } else {
        let script_path = config.get_script_path("parse-legacy-file")?;

        // https://asdf-vm.com/plugins/create.html#bin-parse-legacy-file
        if script_path.exists() {
            let data = exec_script(
                script_path,
                vec![
                    input
                        .path
                        .real_path()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                ],
                FxHashMap::default(),
            )?;

            if !data.is_empty() {
                output.version = Some(UnresolvedVersionSpec::parse(data.trim())?);
            }
        } else {
            output.version = Some(UnresolvedVersionSpec::parse(&input.content)?);
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let download_script_path = config.get_script_path("download")?;
    let install_script_path = config.get_script_path("install")?;

    // In older versions of asdf there may not be a 'download' script,
    // instead both download and install were done in the 'install' script.
    // However, in newer versions, there's two separate 'download' and 'install' scripts.
    let mut env = get_env_vars(&input.context)?;

    // https://asdf-vm.com/plugins/create.html#bin-download
    if download_script_path.exists() {
        exec_script(download_script_path, vec![], env.clone())?;
    } else {
        env.remove("ASDF_DOWNLOAD_PATH");
    }

    // https://asdf-vm.com/plugins/create.html#bin-install
    exec_script(install_script_path, vec![], env)?;

    Ok(Json(NativeInstallOutput {
        installed: true,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn native_uninstall(
    Json(input): Json<NativeUninstallInput>,
) -> FnResult<Json<NativeUninstallOutput>> {
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let script_path = config.get_script_path("uninstall")?;

    // https://asdf-vm.com/plugins/create.html#bin-uninstall
    if script_path.exists() {
        exec_script(script_path, vec![], get_env_vars(&input.context)?)?;
    }

    Ok(Json(NativeUninstallOutput {
        uninstalled: true,
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
        exes: FxHashMap::from_iter([(
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
    let script_path = config.get_script_path("list-all")?;

    //https://asdf-vm.com/plugins/create.html#bin-list-all
    let versions: Vec<String> = exec_bare_script(script_path)?
        .split_whitespace()
        .map(str::to_owned)
        .collect();

    if !versions.is_empty() {
        for version in versions {
            match VersionSpec::parse(version) {
                Ok(version) => output.versions.push(version),
                _ => continue,
            };
        }
    }

    Ok(Json(output))
}
