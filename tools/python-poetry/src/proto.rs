use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;
use std::fs;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: "Poetry".into(),
        type_of: PluginType::CommandLine,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        requires: vec!["python".into()],
        self_upgrade_commands: vec!["self update".into()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".poetry-version".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/python-poetry/poetry")?
        .into_iter()
        .map(|tag| {
            for del in ["a", "b", "rc"] {
                if let Some(parts) = tag.split_once(del) {
                    return format!("{}-{}{}", parts.0, del, parts.1);
                }
            }

            tag
        })
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

        fs::write(&script_path, script)?;
    }

    let result = exec(ExecCommandInput {
        command: "python".into(),
        args: vec![
            script_path.real_path_string().unwrap(),
            "--force".into(),
            "--yes".into(),
        ],
        env: HashMap::from_iter([
            (
                "POETRY_HOME".into(),
                input.install_dir.real_path_string().unwrap(),
            ),
            ("POETRY_VERSION".into(), input.context.version.to_string()),
        ]),
        set_executable: true,
        ..ExecCommandInput::default()
    })?;

    Ok(Json(NativeInstallOutput {
        error: if result.stdout.contains("poetry-installer-error") && result.stdout.contains(".log")
        {
            Some("An error log has been written to the current directory.".into())
        } else {
            None
        },
        installed: result.exit_code == 0,
        ..NativeInstallOutput::default()
    }))
}

#[plugin_fn]
pub fn post_install(Json(input): Json<InstallHook>) -> FnResult<()> {
    let version = input.context.version.as_version();

    // https://python-poetry.org/docs/main/configuration/#virtualenvsuse-poetry-python
    if version.is_some_and(|v| v.major >= 2) {
        exec_captured(
            "poetry",
            ["config", "virtualenvs.use-poetry-python", "false"],
        )?;
    }
    // https://python-poetry.org/docs/1.8/configuration/#virtualenvsprefer-active-python-experimental
    else if version.is_some_and(|v| v.major >= 1 && v.minor >= 2) {
        exec_captured(
            "poetry",
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
        ..LocateExecutablesOutput::default()
    }))
}
