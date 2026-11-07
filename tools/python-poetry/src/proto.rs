use crate::version::*;
use extism_pdk::*;
use lang_python_common::parse_pep440_spec;
use proto_pdk::*;
use starbase_utils::fs;
use std::collections::HashMap;
use toml::Value;
use tool_common::{enable_tracing, extract_version_from_text};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: "Poetry".into(),
        type_of: PluginType::CommandLine,
        lock_options: ToolLockOptions {
            ignore_os_arch: true,
            ..Default::default()
        },
        minimum_proto_version: Some(Version::new(0, 60, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        requires: vec!["python".into()],
        self_upgrade_commands: vec!["self update".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".poetry-version".into(), "pyproject.toml".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == ".poetry-version" {
        if let Some(content) = extract_version_from_text(&input.content) {
            version = Some(UnresolvedVersionSpec::parse(content)?);
        }
    } else if input.file == "pyproject.toml"
        && let Ok(project_toml) = toml::from_str::<Value>(&input.content)
        && let Some(tool_field) = project_toml.get("tool")
        && let Some(poetry_field) = tool_field.get("poetry")
        && let Some(Value::String(constraint)) = poetry_field.get("requires-poetry")
    {
        version = Some(parse_pep440_spec(constraint)?);
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/python-poetry/poetry")?
        .into_iter()
        .map(|tag| from_pep_version(&tag))
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let env = get_host_environment()?;
    let script_path = input.context.temp_dir.join("get-poetry.py");

    if !script_path.exists() {
        let mut script = fetch_text("https://install.python-poetry.org")?;

        // https://stackoverflow.com/a/77120044
        // https://github.com/python-poetry/install.python-poetry.org/issues/24
        if env.os.is_unix() {
            script = script.replace("symlinks=False", "symlinks=True");
        }

        fs::write_file(&script_path, script)?;
    }

    let result = exec(ExecCommandInput {
        command: "python".into(),
        args: vec![
            script_path
                .to_real_path()?
                .expect("Invalid script path!")
                .to_string(),
            "--force".into(),
            "--yes".into(),
        ],
        env: HashMap::from_iter([
            (
                "POETRY_HOME".into(),
                input
                    .install_dir
                    .to_real_path()?
                    .expect("Invalid install directory!")
                    .to_string(),
            ),
            (
                "POETRY_VERSION".into(),
                to_pep_version(&input.context.version.to_string()),
            ),
        ]),
        set_executable: true,
        ..Default::default()
    })?;

    Ok(Json(NativeInstallOutput {
        error: if !result.stdout.is_empty()
            && result.stdout.contains("poetry-installer-error")
            && result.stdout.contains(".log")
        {
            Some("An error log has been written to the current directory.".into())
        } else if !result.stderr.is_empty() {
            Some(result.stderr)
        }
        // The installer reports errors, like an unknown version, on stdout
        else if result.exit_code != 0 {
            Some(result.stdout.trim().to_owned())
        } else {
            None
        },
        installed: result.exit_code == 0,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn post_install(Json(input): Json<InstallHook>) -> FnResult<()> {
    let env = get_host_environment()?;
    let version = input.context.version.as_version();

    // Run the installed version directly, as proto's bins and shims may not be
    // on `PATH` yet, and a shim runs the detected version, not this one
    let poetry = input
        .context
        .tool_dir
        .join(env.os.get_exe_name("bin/poetry"))
        .to_real_path()?
        .expect("Invalid tool directory!")
        .to_string();

    // https://python-poetry.org/docs/main/configuration/#virtualenvsuse-poetry-python
    if version.is_some_and(|v| v.major >= 2) {
        exec_captured(poetry, ["config", "virtualenvs.use-poetry-python", "false"])?;
    }
    // https://python-poetry.org/docs/1.8/configuration/#virtualenvsprefer-active-python-experimental
    else if version.is_some_and(|v| v.major >= 1 && v.minor >= 2) {
        exec_captured(
            poetry,
            ["config", "virtualenvs.prefer-active-python", "true"],
        )?;
    }

    Ok(())
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "poetry".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name("bin/poetry")),
        )]),
        ..Default::default()
    }))
}
