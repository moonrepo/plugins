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
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/python-poetry/poetry")?;

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let script_path = input.context.temp_dir.join("get-poetry.py");

    if !script_path.exists() {
        fs::write(
            &script_path,
            fetch_bytes("https://install.python-poetry.org")?,
        )?;
    }

    let result = exec(ExecCommandInput {
        command: "python".into(),
        args: vec![
            script_path.to_string_lossy().to_string(),
            "--force".into(),
            "--yes".into(),
        ],
        env: HashMap::from_iter([
            (
                "POETRY_HOME".into(),
                into_real_path(input.install_dir.any_path())?
                    .to_string_lossy()
                    .to_string(),
            ),
            ("POETRY_VERSION".into(), input.context.version.to_string()),
            ("PROTO_PYTHON_VERSION".into(), "3".into()),
        ]),
        set_executable: true,
        stream: true,
        ..ExecCommandInput::default()
    })?;

    Ok(Json(NativeInstallOutput {
        installed: result.exit_code == 0,
        ..NativeInstallOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                "uv".into(),
                ExecutableConfig::new_primary(env.os.get_exe_name("uv")),
            ),
            (
                "uvx".into(),
                ExecutableConfig::new(env.os.get_exe_name("uvx")),
            ),
        ]),
        ..LocateExecutablesOutput::default()
    }))
}
