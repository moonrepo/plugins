use crate::config::NpmBackendConfig;
use backend_common::enable_tracing;
use extism_pdk::*;
use proto_pdk::*;
use rustc_hash::FxHashMap;
use schematic::SchemaBuilder;
use starbase_utils::fs;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    let config = get_backend_config::<NpmBackendConfig>()?;

    Ok(Json(RegisterToolOutput {
        name: format!("npm:{}", input.id),
        type_of: PluginType::CommandLine,
        inventory_options: ToolInventoryOptions {
            scoped_backend_dir: true,
            ..Default::default()
        },
        lock_options: ToolLockOptions {
            no_record: true,
            ..Default::default()
        },
        requires: if config.bun {
            vec!["bun".into()]
        } else {
            vec!["node".into(), "npm".into()]
        },
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

#[plugin_fn]
pub fn define_backend_config() -> FnResult<Json<DefineBackendConfigOutput>> {
    Ok(Json(DefineBackendConfigOutput {
        schema: SchemaBuilder::build_root::<NpmBackendConfig>(),
    }))
}

#[plugin_fn]
pub fn native_install(
    Json(input): Json<NativeInstallInput>,
) -> FnResult<Json<NativeInstallOutput>> {
    let config = get_backend_config::<NpmBackendConfig>()?;
    let id = get_plugin_id()?;

    let mut command = ExecCommandInput {
        command: "npm".into(),
        args: vec![
            "install".into(),
            format!("{id}@{}", input.context.version),
            "--global".into(),
        ],
        ..Default::default()
    };

    if config.bun {
        command.command = "bun".into();
        command.args.push("--trust".into());
        command.args.push("--no-save".into());

        // These dirs match the node/npm structure
        command.env.insert(
            "BUN_INSTALL_BIN".into(),
            input.install_dir.join("bin").real_path_string().unwrap(),
        );
        command.env.insert(
            "BUN_INSTALL_GLOBAL_DIR".into(),
            input.install_dir.join("lib").real_path_string().unwrap(),
        );
    } else {
        let install_dir = input.install_dir.real_path_string().unwrap();

        command.args.push("--prefix".into());
        command.args.push(install_dir.clone());
        command.env.insert("PREFIX".into(), install_dir);
    }

    command.cwd = Some(input.install_dir.clone());

    let result = exec(prepare_command(command))?;

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
    let mut output = LocateExecutablesOutput {
        exes_dirs: vec!["bin".into()],
        ..Default::default()
    };
    let mut count = 0;

    for entry in fs::read_dir(input.install_dir.join("bin"))? {
        let name = fs::file_name(entry.path());

        count += 1;
        output
            .exes
            .insert(name.clone(), ExecutableConfig::new(format!("bin/{name}")));
    }

    if count == 1
        && let Some(exe) = output.exes.values_mut().next()
    {
        exe.primary = true;
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn load_versions(Json(input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let config = get_backend_config::<NpmBackendConfig>()?;
    let id = get_plugin_id()?;

    // Create a base command
    let mut command = prepare_command(if config.bun {
        ExecCommandInput::pipe("bun", ["info", &id, "--json"])
    } else {
        ExecCommandInput::pipe("npm", ["view", &id, "--json"])
    });

    // Bun requires a `package.json` in the directory or it fails...
    if config.bun {
        let package_path = input.context.temp_dir.join("package.json");

        if !package_path.exists() {
            fs::write_file(package_path, "{}")?;
        }

        command.cwd = Some(input.context.temp_dir);
    }

    // Fetch versions
    let result = exec({
        let mut cmd = command.clone();
        cmd.args.push("versions".into());
        cmd
    })?;
    let versions: Vec<String> = json::from_str(&result.stdout)?;

    for version in versions {
        output.versions.push(VersionSpec::parse(version.trim())?);
    }

    // Fetch tags
    let result = exec({
        let mut cmd = command.clone();
        cmd.args.push("dist-tags".into());
        cmd
    })?;
    let tags: FxHashMap<String, String> = json::from_str(&result.stdout)?;

    for (alias, version) in tags {
        let version = UnresolvedVersionSpec::parse(&version)?;

        if alias == "latest" {
            output.latest = Some(version.clone());
        }

        output.aliases.entry(alias).or_insert(version);
    }

    Ok(Json(output))
}

fn prepare_command(mut command: ExecCommandInput) -> ExecCommandInput {
    command.env.insert("PROTO_NODE_VERSION?".into(), "*".into());
    command.env.insert("PROTO_NPM_VERSION?".into(), "*".into());
    command.env.insert("PROTO_BUN_VERSION?".into(), "*".into());
    command.stream = false;
    command
}
