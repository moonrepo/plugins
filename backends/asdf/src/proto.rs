use crate::config::AsdfPluginConfig;
use extism_pdk::*;
use proto_pdk::*;
use starbase_utils::fs;
use std::path::Path;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn from_virtual_path(path: String) -> String;
    fn to_virtual_path(path: String) -> String;
    fn host_log(input: Json<HostLogInput>);
}

fn cpu_cores() -> AnyResult<String> {
    if let Some(value) = var::get("cpu_count")? {
        return Ok(value);
    }

    let result = if get_host_environment()?.os.is_mac() {
        exec_captured("sysctl", ["-n", "hw.physicalcpu"])?
    } else {
        exec_captured("nproc", Vec::<String>::new())?
    };

    let value = result.stdout.trim().to_string();

    var::set("cpu_count", &value)?;

    Ok(value)
}

fn create_script(virtual_script_path: &Path, context: &ToolContext) -> AnyResult<ExecCommandInput> {
    if !virtual_script_path.exists() {
        return Err(PluginError::Message(format!(
            "Script <id>{}</id> not found. Is the asdf repository valid?",
            fs::file_name(virtual_script_path)
        ))
        .into());
    }

    let mut input = ExecCommandInput {
        command: "bash".into(),
        working_dir: Some(context.tool_dir.clone()),
        ..Default::default()
    };

    input.args.push(
        into_real_path(virtual_script_path)?
            .to_string_lossy()
            .to_string(),
    );

    input
        .env
        .insert("ASDF_INSTALL_TYPE".into(), "version".into());
    input
        .env
        .insert("ASDF_INSTALL_VERSION".into(), context.version.to_string());
    input.env.insert(
        "ASDF_INSTALL_PATH".into(),
        context
            .tool_dir
            .real_path()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    );
    input.env.insert(
        "ASDF_DOWNLOAD_PATH".into(),
        context
            .temp_dir
            .real_path()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    );

    Ok(input)
}

fn exec_script(input: ExecCommandInput) -> AnyResult<String> {
    let script_path = input.args[0].clone();
    let result = exec(input)?;

    if result.exit_code != 0 {
        return Err(PluginError::Message(format!(
            "Failed to execute script <path>{script_path}</path>: {}",
            result.stderr
        ))
        .into());
    }

    Ok(result.stdout)
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: format!("asdf:{}", input.id),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        config_schema: Some(schematic::SchemaBuilder::generate::<AsdfPluginConfig>()),
        unstable: Switch::Message("asdf backend is experimental. Any tools that require <id>exec-env</id> may not work correctly. Please report any issues.".into()),
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
        exes: vec![
            "bin/download".into(),
            "bin/install".into(),
            "bin/list-all".into(),
            "bin/list-bin-paths".into(),
            "bin/uninstall".into(),
            "bin/list-legacy-filenames".into(),
            "bin/parse-legacy-file".into(),
        ],
        source: Some(SourceLocation::Git(GitSource {
            url: config.get_repo_url()?,
            ..GitSource::default()
        })),
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
        let data = exec_script(create_script(&script_path, &input.context)?)?;

        for file in data.split_whitespace() {
            output.files.push(file.into());
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
            let mut script = create_script(&script_path, &input.context)?;
            script.env.clear();
            script.args.push(
                input
                    .path
                    .real_path()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            );

            let data = exec_script(script)?;

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

    // In older versions of asdf there may not be a 'download' script,
    // instead both download and install were done in the 'install' script.
    // However, in newer versions, there's two separate 'download' and 'install' scripts.
    let download_script_path = config.get_script_path("download")?;
    let install_script_path = config.get_script_path("install")?;

    // https://asdf-vm.com/plugins/create.html#bin-download
    if download_script_path.exists() {
        exec_script(create_script(&download_script_path, &input.context)?)?;
    }

    // https://asdf-vm.com/plugins/create.html#bin-install
    let mut script = create_script(&install_script_path, &input.context)?;
    script.env.insert("ASDF_CONCURRENCY".into(), cpu_cores()?);

    exec_script(script)?;

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
        exec_script(create_script(&script_path, &input.context)?)?;
    }

    Ok(Json(NativeUninstallOutput {
        uninstalled: true,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let mut output = LocateExecutablesOutput::default();
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let script_path = config.get_script_path("list-bin-paths")?;

    // https://asdf-vm.com/plugins/create.html#bin-list-bin-paths
    if script_path.exists() {
        let data = exec_script(create_script(&script_path, &input.context)?)?;

        for dir in data.split_whitespace() {
            output.exes_dirs.push(dir.into());
        }
    } else {
        output.exes_dirs.push("bin".into());
    }

    if let Some(dir) = output.exes_dirs.first() {
        let id = config.get_id()?;

        for entry in fs::read_dir(input.context.tool_dir.join(dir))? {
            let file = entry.path();
            let name = fs::file_name(&file);

            output.exes.insert(
                name.clone(),
                ExecutableConfig {
                    primary: name == id,
                    exe_path: match file.strip_prefix(&input.context.tool_dir) {
                        Ok(suffix) => Some(suffix.to_owned()),
                        Err(_) => Some(dir.join(name)),
                    },
                    ..Default::default()
                },
            );
        }
    }

    if output.exes.is_empty() {
        let exe = config.get_exe_name()?;

        output.exes.insert(
            exe.clone(),
            ExecutableConfig::new_primary(format!("bin/{exe}")),
        );
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn load_versions(Json(input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let config = get_tool_config::<AsdfPluginConfig>()?;
    let script_path = config.get_script_path("list-all")?;

    //https://asdf-vm.com/plugins/create.html#bin-list-all
    let mut script = create_script(&script_path, &input.context)?;
    script.env.clear();
    script.working_dir = None;

    let data = exec_script(script)?;

    for version in data.split_whitespace() {
        match VersionSpec::parse(version.trim()) {
            Ok(version) => output.versions.push(version),
            _ => continue,
        };
    }

    Ok(Json(output))
}
