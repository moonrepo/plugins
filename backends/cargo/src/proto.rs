use backend_common::enable_tracing;
use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use starbase_utils::fs;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: format!("cargo:{}", input.id),
        type_of: PluginType::CommandLine,
        inventory_options: ToolInventoryOptions {
            scoped_backend_dir: !input.id.starts_with("cargo-"),
            ..Default::default()
        },
        lock_options: ToolLockOptions {
            no_record: true,
            ..Default::default()
        },
        requires: vec!["rust".into()],
        minimum_proto_version: Some(Version::new(0, 53, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Toggle(true),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn register_backend(
    Json(_input): Json<RegisterBackendInput>,
) -> FnResult<Json<RegisterBackendOutput>> {
    Ok(Json(RegisterBackendOutput {
        backend_id: get_plugin_id()?,
        ..Default::default()
    }))
}

fn get_cargo_home(env: &HostEnvironment) -> Result<VirtualPath, Error> {
    let dir = env.home_dir.join(".cargo");

    match get_host_env_var("CARGO_HOME")? {
        Some(value) => Ok(if value.is_empty() {
            dir
        } else {
            into_virtual_path(value)?
        }),
        None => Ok(dir),
    }
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let id = get_plugin_id()?;
    let env = get_host_environment()?;

    // Detect `cargo-binstall`
    let cargo_home_dir = get_cargo_home(&env)?;
    let binstall_path = cargo_home_dir
        .join("bin")
        .join(env.os.get_exe_name("cargo-binstall"));
    let use_binstall = binstall_path.exists() && binstall_path.is_file();

    // Create the command
    let mut command =
        ExecCommandInput::pipe("cargo", [if use_binstall { "binstall" } else { "install" }]);
    command.cwd = Some(input.install_dir.clone());
    command.env.insert("PROTO_RUST_VERSION?".into(), "*".into());

    // What to install
    command.args.push(format!("{id}@{}", input.context.version));

    if use_binstall {
        command.args.push("--no-confirm".into());
    }

    // Where to install
    command.args.push("--root".into());
    command
        .args
        .push(input.install_dir.real_path_string().unwrap());

    // Other options
    if input.force {
        command.args.push("--force".into());
    }

    let result = exec(command)?;

    Ok(Json(NativeInstallOutput {
        installed: result.exit_code == 0,
        error: if result.stderr.is_empty() {
            None
        } else {
            Some(result.stderr)
        },
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let id = get_plugin_id()?;
    let mut output = LocateExecutablesOutput {
        exes_dirs: vec!["bin".into()],
        ..Default::default()
    };
    let mut count = 0;

    for entry in fs::read_dir(input.install_dir.join("bin"))? {
        let name = fs::file_name(entry.path());

        let mut config = ExecutableConfig::new(format!("bin/{name}"));
        config.primary = id == name;

        count += 1;
        output.exes.insert(name.clone(), config);
    }

    if count == 1
        && let Some(exe) = output.exes.values_mut().next()
    {
        exe.primary = true;
    }

    Ok(Json(output))
}

#[derive(Deserialize)]
struct VersionItem {
    #[serde(alias = "vers")]
    num: String,
    yanked: bool,
}

// #[derive(Deserialize)]
// struct VersionsMeta {
// }

#[derive(Deserialize)]
struct VersionsResponse {
    versions: Vec<VersionItem>,
    // Handle next page?
    // meta: VersionsMeta,
}

#[plugin_fn]
pub fn load_versions(Json(_input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let id = get_plugin_id()?;

    let response: VersionsResponse =
        fetch_json(format!("https://crates.io/api/v1/crates/{id}/versions"))?;

    for version in response.versions {
        if !version.yanked {
            output.versions.push(VersionSpec::parse(version.num)?);
        }
    }

    Ok(Json(output))
}
