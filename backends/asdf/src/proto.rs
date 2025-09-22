use crate::config::AsdfToolConfig;
use backend_common::enable_tracing;
use extism_pdk::*;
use proto_pdk::*;
use rustc_hash::FxHashMap;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use std::path::{Path, PathBuf};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn from_virtual_path(path: String) -> String;
    fn to_virtual_path(path: String) -> String;
    fn host_log(input: Json<HostLogInput>);
}

// The plugin is being used as-is without a "tool" assigned to it,
// so return nothing instead of failing. This happens during
// `proto plugin info` and other similar commands.
fn is_asdf() -> bool {
    get_plugin_id().is_ok_and(|id| id == "asdf")
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

fn backend_root() -> AnyResult<PathBuf> {
    if let Some(value) = var::get::<String>("backend_root")? {
        return Ok(value.into());
    }

    let root = into_real_path("/proto/backends")?;

    var::set("backend_root", root.to_str().unwrap())?;

    Ok(root)
}

fn create_script_from_context(
    virtual_script_path: &Path,
    context: &PluginContext,
) -> AnyResult<ExecCommandInput> {
    create_script(
        virtual_script_path,
        Some(&context.version),
        Some(&context.tool_dir),
        Some(&context.temp_dir),
    )
}

fn create_script_from_unresolved_context(
    virtual_script_path: &Path,
    context: &PluginUnresolvedContext,
) -> AnyResult<ExecCommandInput> {
    create_script(virtual_script_path, context.version.as_ref(), None, None)
}

fn create_script(
    virtual_script_path: &Path,
    version: Option<&VersionSpec>,
    tool_dir: Option<&VirtualPath>,
    temp_dir: Option<&VirtualPath>,
) -> AnyResult<ExecCommandInput> {
    if !virtual_script_path.exists() {
        return Err(PluginError::Message(format!(
            "Script <id>{}</id> not found. Is the asdf repository valid?",
            fs::file_name(virtual_script_path)
        ))
        .into());
    }

    let mut shell = "bash".to_owned();

    // Extract the shell to use from the shebang
    if let Ok(script_contents) = fs::read_file(virtual_script_path)
        && let Some(line) = script_contents.lines().next()
        && line.starts_with("#!")
    {
        let mut parts = line.trim().split(' ');

        if let Some(last) = parts.next_back() {
            shell = last.to_owned();
        }
    }

    let mut input = ExecCommandInput {
        command: shell,
        cwd: tool_dir.cloned(),
        ..Default::default()
    };

    // Resolve the real path since this is executed in the console
    input.args.push(
        match virtual_script_path.strip_prefix("/proto/backends") {
            Ok(suffix) => backend_root()?.join(suffix),
            Err(_) => into_real_path(virtual_script_path)?,
        }
        .to_string_lossy()
        .to_string(),
    );

    if let Some(version) = version {
        input
            .env
            .insert("ASDF_INSTALL_TYPE".into(), "version".into());
        input
            .env
            .insert("ASDF_INSTALL_VERSION".into(), version.to_string());
    }

    if let Some(dir) = tool_dir {
        input
            .env
            .insert("ASDF_INSTALL_PATH".into(), dir.real_path_string().unwrap());
    }

    if let Some(dir) = temp_dir {
        input
            .env
            .insert("ASDF_DOWNLOAD_PATH".into(), dir.real_path_string().unwrap());
    }

    Ok(input)
}

fn exec_script(input: ExecCommandInput) -> AnyResult<String> {
    let script_path = input.args[0].clone();
    let result = exec(input)?;

    handle_exec_result(script_path, result)
}

fn handle_exec_result(script_path: String, result: ExecCommandOutput) -> AnyResult<String> {
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
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: if input.id == "asdf" {
            input.id.to_string()
        } else {
            format!("asdf:{}", input.id)
        },
        type_of: if input.id == "asdf" {
            PluginType::VersionManager
        } else {
            PluginType::Language
        },
        lock_options: ToolLockOptions {
            no_record: true,
            ..Default::default()
        },
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Toggle(true),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<AsdfToolConfig>(),
    }))
}

#[plugin_fn]
pub fn register_backend(
    Json(input): Json<RegisterBackendInput>,
) -> FnResult<Json<RegisterBackendOutput>> {
    if get_host_environment()?.os.is_windows() {
        return Err(PluginError::UnsupportedOS {
            tool: input.id.to_string(),
            os: "windows".into(),
        }
        .into());
    }

    let config = get_tool_config::<AsdfToolConfig>()?;

    Ok(Json(RegisterBackendOutput {
        backend_id: config.get_backend_id()?,
        exes: vec![
            "bin/download".into(),
            "bin/exec-env".into(),
            "bin/install".into(),
            "bin/latest-stable".into(),
            "bin/list-all".into(),
            "bin/list-bin-paths".into(),
            "bin/list-legacy-filenames".into(),
            "bin/parse-legacy-file".into(),
            "bin/uninstall".into(),
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

    if is_asdf() {
        return Ok(Json(output));
    }

    let config = get_tool_config::<AsdfToolConfig>()?;
    let script_path = config.get_script_path("list-legacy-filenames")?;

    output.files = vec![".tool-versions".into()];

    // https://asdf-vm.com/plugins/create.html#bin-list-legacy-filenames
    if script_path.exists() {
        let data = exec_script(create_script_from_unresolved_context(
            &script_path,
            &input.context,
        )?)?;

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
    let config = get_tool_config::<AsdfToolConfig>()?;

    if input.file == ".tool-versions" {
        let id = get_plugin_id()?;

        for line in input.content.lines() {
            let mut parsed_line = String::new();

            // Strip comments
            for ch in line.chars() {
                if ch == '#' {
                    break;
                }

                parsed_line.push(ch);
            }

            let (tool, version) = parsed_line.split_once(' ').unwrap_or((&parsed_line, ""));

            if id == tool && !version.is_empty() {
                output.version = Some(UnresolvedVersionSpec::parse(version)?);
                break;
            }
        }
    } else {
        let script_path = config.get_script_path("parse-legacy-file")?;

        // https://asdf-vm.com/plugins/create.html#bin-parse-legacy-file
        if script_path.exists() {
            let mut script = create_script_from_unresolved_context(&script_path, &input.context)?;
            script.args.push(input.path.real_path_string().unwrap());

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
    if is_asdf() {
        return Ok(Json(NativeInstallOutput {
            error: Some("asdf itself cannot be installed, only asdf plugins.".into()),
            installed: false,
            ..Default::default()
        }));
    }

    let config = get_tool_config::<AsdfToolConfig>()?;

    // In older versions of asdf there may not be a 'download' script,
    // instead both download and install were done in the 'install' script.
    // However, in newer versions, there's two separate 'download' and 'install' scripts.
    let download_script_path = config.get_script_path("download")?;
    let install_script_path = config.get_script_path("install")?;

    // https://asdf-vm.com/plugins/create.html#bin-download
    if download_script_path.exists() {
        exec_script(create_script_from_context(
            &download_script_path,
            &input.context,
        )?)?;
    }

    // https://asdf-vm.com/plugins/create.html#bin-install
    let mut script = create_script_from_context(&install_script_path, &input.context)?;
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
    let config = get_tool_config::<AsdfToolConfig>()?;
    let script_path = config.get_script_path("uninstall")?;

    // https://asdf-vm.com/plugins/create.html#bin-uninstall
    if script_path.exists() {
        exec_script(create_script_from_context(&script_path, &input.context)?)?;
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

    if is_asdf() {
        return Ok(Json(output));
    }

    let config = get_tool_config::<AsdfToolConfig>()?;
    let script_path = config.get_script_path("list-bin-paths")?;

    // https://asdf-vm.com/plugins/create.html#bin-list-bin-paths
    if script_path.exists() {
        let data = exec_script(create_script_from_context(&script_path, &input.context)?)?;

        for dir in data.split_whitespace() {
            output.exes_dirs.push(dir.into());
        }
    } else {
        output.exes_dirs.push("bin".into());
    }

    let id = get_plugin_id()?;

    if let Some(exes) = config.exes {
        for exe in exes {
            output.exes.insert(
                exe.clone(),
                ExecutableConfig {
                    primary: id == exe,
                    exe_path: Some(format!("bin/{exe}").into()),
                    ..Default::default()
                },
            );
        }
    } else if let Some(dir) = output.exes_dirs.first() {
        for entry in fs::read_dir(input.install_dir.join(dir))? {
            let file = entry.path();
            let name = fs::file_name(&file);

            // Some asdf plugins just unpack the entire repository/archive
            // into the same folder, flooding it with non-sense files. Let's
            // do our best to filter them out...
            if name.contains("README")
                || name.contains("HISTORY")
                || name.contains("CHANGELOG")
                || name.contains("LICENSE")
                || name.starts_with('.')
            {
                continue;
            }

            // Let's also do some filtering based on file extension
            match file.extension().and_then(|ext| ext.to_str()) {
                Some("sh" | "exe") | None => {
                    // Allowed
                }
                Some(_) => {
                    // Unknown extension, not allowed
                    continue;
                }
            };

            output.exes.insert(
                name.clone(),
                ExecutableConfig {
                    primary: id == name,
                    exe_path: match file.strip_prefix(&input.install_dir) {
                        Ok(suffix) => Some(suffix.to_owned()),
                        Err(_) => Some(dir.join(name)),
                    },
                    ..Default::default()
                },
            );
        }
    }

    // Return at least something!
    if output.exes.is_empty() {
        output.exes.insert(
            id.to_string(),
            ExecutableConfig::new_primary(format!("bin/{id}")),
        );
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn load_versions(Json(input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();

    if is_asdf() {
        return Ok(Json(output));
    }

    let config = get_tool_config::<AsdfToolConfig>()?;
    let script_path = config.get_script_path("list-all")?;

    // https://asdf-vm.com/plugins/create.html#bin-list-all
    let data = exec_script(create_script_from_unresolved_context(
        &script_path,
        &input.context,
    )?)?;

    for version in data.split_whitespace() {
        match VersionSpec::parse(version.trim()) {
            Ok(version) => output.versions.push(version),
            _ => continue,
        };
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    if let UnresolvedVersionSpec::Alias(alias) = input.initial
        && alias == "stable"
    {
        let config = get_tool_config::<AsdfToolConfig>()?;
        let script_path = config.get_script_path("latest-stable")?;

        // https://asdf-vm.com/plugins/create.html#bin-latest-stable
        if script_path.exists() {
            let data = exec_script(create_script_from_unresolved_context(
                &script_path,
                &input.context,
            )?)?;

            if !data.is_empty() {
                output.candidate = UnresolvedVersionSpec::parse(data.trim()).ok();
            }
        } else {
            output.candidate = Some(UnresolvedVersionSpec::Alias("latest".into()));
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn pre_run(Json(input): Json<RunHook>) -> FnResult<Json<RunHookResult>> {
    let mut output = RunHookResult::default();
    let config = get_tool_config::<AsdfToolConfig>()?;
    let script_path = config.get_script_path("exec-env")?;

    // https://asdf-vm.com/plugins/create.html#bin-exec-env
    if !script_path.exists() {
        return Ok(Json(output));
    }

    // Because `exec-env` sets environment variables, we simply can't execute
    // the script as-is, we need to execute it and capture the variables that were
    // set. This is complicated, but doable. We will achieve this by running `env`
    // to capture the existing variables, then `source`ing the `exec-env` script,
    // then running `env` again to capture the new/different variables.
    let mut script = create_script_from_context(&script_path, &input.context)?;
    let real_script_path = script.args.remove(0);

    script.args.push("-c".into());
    script.args.push(format!(
        "env; echo \"###\"; source \"{real_script_path}\"; env;"
    ));

    let stdout = handle_exec_result(real_script_path, exec(script)?)?;
    let mut existing_env = FxHashMap::default();
    let mut after_source = false;

    for line in stdout.lines() {
        if line == "###" {
            after_source = true;
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            if after_source {
                if let Some(existing_value) = existing_env.get(key)
                    && value == *existing_value
                {
                    continue;
                }

                output
                    .env
                    .get_or_insert_default()
                    .insert(key.to_owned(), value.to_owned());
            } else {
                existing_env.insert(key, value);
            }
        }
    }

    Ok(Json(output))
}
