use backend_common::enable_tracing;
use extism_pdk::*;
use proto_pdk::*;
use rustc_hash::FxHashMap;
use serde::Deserialize;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    // fn from_virtual_path(path: String) -> String;
    // fn to_virtual_path(path: String) -> String;
    // fn host_log(input: Json<HostLogInput>);
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: if input.id == "npm" {
            input.id.clone()
        } else {
            format!("npm:{}", input.id)
        },
        type_of: if input.id == "npm" {
            PluginType::DependencyManager
        } else {
            PluginType::CommandLine
        },
        lock_options: ToolLockOptions {
            no_record: true,
            ..Default::default()
        },
        requires: vec!["node".into(), "npm".into()],
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
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

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let id = get_plugin_id()?;
    let install_dir = input.install_dir.real_path_string().unwrap();

    let mut command = ExecCommandInput {
        command: "npm".into(),
        args: vec![
            "install".into(),
            format!("{id}@{}", input.context.version),
            "--global".into(),
            "--prefix".into(),
            install_dir.clone(),
        ],
        ..Default::default()
    };

    command.env.insert("PREFIX".into(), install_dir);
    command.cwd = Some(input.install_dir.clone());

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
pub fn native_uninstall(
    Json(input): Json<NativeUninstallInput>,
) -> FnResult<Json<NativeUninstallOutput>> {
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

    Ok(Json(output))
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct NpmViewInfo {
    #[serde(alias = "dist-tags")]
    dist_tags: FxHashMap<String, String>,
    versions: Vec<String>,
}

#[plugin_fn]
pub fn load_versions(Json(_input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let id = get_plugin_id()?;

    let result = exec_captured("npm", ["view", &id, "--json"])?;
    let info: NpmViewInfo = json::from_str(&result.stdout)?;

    for version in info.versions {
        output.versions.push(VersionSpec::parse(version.trim())?);
    }

    for (alias, version) in info.dist_tags {
        let version = UnresolvedVersionSpec::parse(&version)?;

        if alias == "latest" {
            output.latest = Some(version.clone());
        }

        output.aliases.entry(alias).or_insert(version);
    }

    Ok(Json(output))
}
