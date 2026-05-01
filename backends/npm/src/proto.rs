use crate::config::NpmBackendConfig;
use backend_common::enable_tracing;
use extism_pdk::*;
use proto_pdk::*;
use rustc_hash::FxHashMap;
use schematic::SchemaBuilder;
use starbase_utils::{fs, json::JsonValue};

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
    let mut output = LocateExecutablesOutput::default();

    let package_name = get_plugin_id()?;
    let package_name_without_scope = match package_name.rfind('/') {
        Some(index) => &package_name[index + 1..],
        None => &package_name,
    };

    let rel_package_path = format!(
        "{}/{package_name}",
        // Differences between unix/windows and node/bun
        if input.install_dir.join("lib").exists() {
            "lib/node_modules"
        } else {
            "node_modules"
        }
    );
    let package_json_path = input
        .install_dir
        .join(&rel_package_path)
        .join("package.json");

    let create_exe_config = |bin_path: &str, primary: bool| {
        let mut config = ExecutableConfig::new(format!(
            "{rel_package_path}/{}",
            bin_path.trim_start_matches("./")
        ));

        config.primary = primary;

        if bin_path.ends_with(".js") || bin_path.ends_with(".cjs") || bin_path.ends_with(".mjs") {
            config.parent_exe_name = Some("node".into());
            config.no_bin = true;
        } else if bin_path.ends_with(".ts")
            || bin_path.ends_with(".cts")
            || bin_path.ends_with(".mts")
            || bin_path.ends_with(".tsx")
        {
            config.parent_exe_name = Some("tsx".into());
            config.no_bin = true;
        }

        config
    };

    // If the package exists, extract the applicable bins from it
    if package_json_path.exists() {
        let package: JsonValue = starbase_utils::json::read_file(package_json_path)?;

        match package.get("bin") {
            Some(JsonValue::Object(bins)) => {
                for (i, (bin, bin_path)) in bins.iter().enumerate() {
                    if let JsonValue::String(bin_path) = bin_path {
                        output
                            .exes
                            .insert(bin.into(), create_exe_config(bin_path, i == 0));
                    }
                }
            }
            Some(JsonValue::String(bin_path)) => {
                output.exes.insert(
                    package_name_without_scope.into(),
                    create_exe_config(bin_path, true),
                );
            }
            _ => {}
        };
    }

    // Otherwise, scan the file system
    if output.exes.is_empty() {
        // Differences between unix/windows and node/bun
        let bin_dir = if input.install_dir.join("bin").exists() {
            input.install_dir.join("bin")
        } else {
            input.install_dir.clone()
        };

        for entry in fs::read_dir(bin_dir)? {
            // Windows contains `.cmd` and `.ps1` that we should avoid
            if !entry.path().is_file() || entry.path().extension().is_some() {
                continue;
            }

            let name = fs::file_name(entry.path());
            let mut config = ExecutableConfig::new(
                entry
                    .path()
                    .strip_prefix(&input.install_dir)
                    .unwrap()
                    .to_string_lossy(),
            );

            if name == package_name_without_scope {
                config.primary = true;
            }

            output.exes.insert(name, config);
        }
    }

    // This is dangerous but we need a primary!
    if !output.exes.iter().any(|(_, cfg)| cfg.primary)
        && let Some(exe) = output.exes.values_mut().next()
    {
        exe.primary = true;
    }

    // Support activate flows
    output
        .exes_dirs
        .push(if input.install_dir.join("bin").exists() {
            "bin".into()
        } else {
            ".".into()
        });

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
