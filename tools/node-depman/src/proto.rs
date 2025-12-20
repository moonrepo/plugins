use crate::config::NodeDepmanToolConfig;
use crate::npm_registry::parse_registry_response;
use crate::package_manager::PackageManager;
use extism_pdk::*;
use lang_javascript_common::{
    NodeDistVersion, extract_dev_engine_package_manager_version, extract_engine_version,
    extract_package_manager_version, extract_volta_version,
};
use nodejs_package_json::PackageJson;
use proto_pdk::*;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use std::collections::HashMap;
use tool_common::enable_tracing;

const BASH_SHIM_TEMPLATE: &str = include_str!("../templates/bash-shim.sh");
const CMD_SHIM_TEMPLATE: &str = include_str!("../templates/cmd-shim.cmd");

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn get_env_var(key: &str) -> String;
    fn to_virtual_path(input: String) -> String;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    let manager = PackageManager::detect()?;

    Ok(Json(RegisterToolOutput {
        name: manager.to_string(),
        type_of: PluginType::DependencyManager,
        default_version: if manager == PackageManager::Npm {
            Some(UnresolvedVersionSpec::Alias("bundled".into()))
        } else {
            None
        },
        lock_options: ToolLockOptions {
            ignore_os_arch: true,
            ..Default::default()
        },
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        requires: vec!["node".into()],
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<NodeDepmanToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec!["package.json".into()],
        ignore: vec!["node_modules".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == "package.json"
        && let Ok(package_json) = json::from_str::<PackageJson>(&input.content)
    {
        let manager_name = PackageManager::detect()?.to_string();

        if let Some(constraint) =
            extract_dev_engine_package_manager_version(&package_json, &manager_name)
        {
            version = Some(UnresolvedVersionSpec::parse(constraint)?);
        }

        if version.is_none()
            && let Some(constraint) = extract_package_manager_version(&package_json, &manager_name)
        {
            version = Some(UnresolvedVersionSpec::parse(constraint)?);
        }

        if version.is_none()
            && let Some(constraint) =
                extract_volta_version(&package_json, &input.path, &manager_name)?
        {
            version = Some(UnresolvedVersionSpec::parse(constraint)?);
        }

        if version.is_none()
            && let Some(constraint) = extract_engine_version(&package_json, &manager_name)
        {
            version = Some(UnresolvedVersionSpec::parse(constraint)?);
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let manager = PackageManager::detect()?;
    let registry_url = get_tool_config::<NodeDepmanToolConfig>()?.registry_url;
    let package_name = manager.get_package_name(&input.initial);

    let mut map_output = |res_text: String, is_yarn: bool| -> Result<(), Error> {
        let res = parse_registry_response(res_text, is_yarn)?;

        for item in res.versions.values() {
            output.versions.push(VersionSpec::parse(&item.version)?);
        }

        // Dist tags always includes latest
        for (alias, version) in res.dist_tags {
            let version = UnresolvedVersionSpec::parse(&version)?;

            if alias == "latest" {
                output.latest = Some(version.clone());

                // The berry alias only exists in the `yarn` package,
                // but not `@yarnpkg/cli-dist`, so update it here
                if is_yarn && res.name == "@yarnpkg/cli-dist" {
                    output.aliases.insert("berry".into(), version.clone());
                }
            }

            output.aliases.entry(alias).or_insert(version);
        }

        Ok(())
    };

    // Yarn is managed by 2 different packages, so we need to request versions from both of them!
    if manager == PackageManager::Yarn {
        map_output(fetch_text(format!("{registry_url}/yarn/"))?, true)?;
        map_output(
            fetch_text(format!("{registry_url}/@yarnpkg/cli-dist/"))?,
            true,
        )?;
    } else {
        map_output(
            fetch_text(format!("{registry_url}/{package_name}/"))?,
            false,
        )?;
    }

    output
        .aliases
        .insert("latest".into(), output.latest.clone().unwrap());

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let manager = PackageManager::detect()?;
    let mut output = ResolveVersionOutput::default();

    match manager {
        PackageManager::Npm => {
            // When the alias "bundled" is provided, we should install the npm
            // version that comes bundled with the current Node.js version.
            if input.initial.is_alias("bundled") {
                debug!("Received the bundled alias, attempting to find a version");

                let response: Vec<NodeDistVersion> =
                    fetch_json("https://nodejs.org/download/release/index.json")?;
                let mut found_version = false;

                // Infer from proto's environment variable
                if let Some(node_version) = get_host_env_var("PROTO_NODE_VERSION")? {
                    for node_release in &response {
                        // Theirs starts with v, ours does not
                        if node_release.version[1..] == node_version && node_release.npm.is_some() {
                            output.version =
                                Some(VersionSpec::parse(node_release.npm.as_ref().unwrap())?);
                            found_version = true;
                            break;
                        }
                    }
                }

                // Otherwise call the current `node` binary and infer from that
                if !found_version {
                    let result = exec_captured("node", ["--version"])?;
                    let node_version = result.stdout.trim();

                    for node_release in &response {
                        // Both start with v
                        if node_release.version == node_version && node_release.npm.is_some() {
                            output.version =
                                Some(VersionSpec::parse(node_release.npm.as_ref().unwrap())?);
                            found_version = true;
                            break;
                        }
                    }
                }

                if !found_version {
                    debug!(
                        "Could not find a bundled npm version for Node.js, falling back to latest"
                    );

                    output.candidate = Some(UnresolvedVersionSpec::Alias("latest".into()));
                }
            }
        }

        PackageManager::Yarn => {
            if let UnresolvedVersionSpec::Alias(alias) = input.initial {
                if alias == "berry" || alias == "latest" {
                    output.candidate = Some(UnresolvedVersionSpec::parse("~4")?);
                } else if alias == "legacy" || alias == "classic" {
                    output.candidate = Some(UnresolvedVersionSpec::parse("~1")?);
                }
            }
        }

        _ => {}
    };

    Ok(Json(output))
}

fn get_archive_prefix(manager: &PackageManager, spec: &VersionSpec) -> String {
    if manager.is_yarn_classic(spec.to_unresolved_spec())
        && let Some(version) = spec.as_version()
    {
        // Prefix changed to "package" in v1.22.20
        // https://github.com/yarnpkg/yarn/releases/tag/v1.22.20
        if version.minor <= 22 && version.patch <= 19 {
            return format!("yarn-v{version}");
        }
    }

    "package".into()
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let version = &input.context.version;
    let manager = PackageManager::detect()?;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: manager.to_string()
        }));
    }

    let package_name = manager.get_package_name(version.to_unresolved_spec());

    let package_without_scope = if let Some(index) = package_name.find('/') {
        &package_name[index + 1..]
    } else {
        &package_name
    };

    let registry_url = get_tool_config::<NodeDepmanToolConfig>()?.registry_url;
    let dist_url = get_tool_config::<NodeDepmanToolConfig>()?.dist_url;
    let filename = format!("{package_without_scope}-{version}.tgz");

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(get_archive_prefix(&manager, version)),
        download_url: dist_url
            .replace("{registry}", &registry_url)
            .replace("{package}", &package_name)
            .replace("{package_without_scope}", package_without_scope)
            .replace("{version}", &version.to_string())
            .replace("{file}", &filename),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let manager = PackageManager::detect()?;
    let mut secondary = HashMap::<String, ExecutableConfig>::default();
    let primary;

    if !input.install_dir.join("shims").exists() {
        create_internal_shims(&env, &input.install_dir, &manager)?;
    }

    // These are the directories that contain the executable binaries,
    // NOT where the packages/node modules are stored. Some package managers
    // have separate folders for the 2 processes, and then create symlinks.
    let mut globals_lookup_dirs = vec![
        "$PREFIX/shims".into(),
        "$PROTO_HOME/tools/node/$PROTO_NODE_VERSION/bin".into(),
    ];

    match &manager {
        PackageManager::Npm => {
            primary = ExecutableConfig::new_primary("shims/npm");

            // npx
            secondary.insert("npx".into(), ExecutableConfig::new("shims/npx"));

            // node-gyp
            secondary.insert(
                "node-gyp".into(),
                ExecutableConfig::with_parent("node_modules/node-gyp/bin/node-gyp.js", "node"),
            );

            // https://docs.npmjs.com/cli/v9/configuring-npm/folders#prefix-configuration
            // https://github.com/npm/cli/blob/latest/lib/npm.js
            // https://github.com/npm/cli/blob/latest/workspaces/config/lib/index.js#L339
            globals_lookup_dirs.push("$TOOL_DIR/shims".into());
        }
        PackageManager::Pnpm => {
            primary = ExecutableConfig::new_primary("shims/pnpm");

            // pnpx
            secondary.insert("pnpx".into(), ExecutableConfig::new("shims/pnpx"));

            // https://pnpm.io/npmrc#global-dir
            // https://github.com/pnpm/pnpm/blob/main/config/config/src/index.ts#L350
            // https://github.com/pnpm/pnpm/blob/main/config/config/src/dirs.ts#L40
            globals_lookup_dirs.push("$PNPM_HOME".into());

            if env.os.is_windows() {
                globals_lookup_dirs.push("$LOCALAPPDATA\\pnpm".into());
            } else if env.os.is_mac() {
                globals_lookup_dirs.push("$HOME/Library/pnpm".into());
            } else {
                globals_lookup_dirs.push("$HOME/.local/share/pnpm".into());
            }
        }
        PackageManager::Yarn => {
            primary = ExecutableConfig::new_primary("shims/yarn");

            // yarnpkg
            secondary.insert("yarnpkg".into(), ExecutableConfig::new("shims/yarn"));

            // https://github.com/yarnpkg/yarn/blob/master/src/cli/commands/global.js#L84
            if env.os.is_windows() {
                globals_lookup_dirs.push("$LOCALAPPDATA\\Yarn\\bin".into());
                globals_lookup_dirs.push("$HOME\\.yarn\\bin".into());
            } else {
                globals_lookup_dirs.push("$HOME/.yarn/bin".into());
            }
        }
    };

    let config = get_tool_config::<NodeDepmanToolConfig>()?;

    if config.shared_globals_dir {
        globals_lookup_dirs.clear();
        globals_lookup_dirs.push("$PROTO_HOME/tools/node/globals/bin".into());
    }

    let mut exes = HashMap::from_iter([(manager.to_string(), primary)]);
    exes.extend(secondary);

    // Update the permissions of each executable since they are custom shims
    exes.iter_mut().for_each(|(name, config)| {
        config.no_bin = true;

        if name != "node-gyp" {
            config.update_perms = true;
        }

        if env.os.is_windows()
            && let Some(exe_path) = &mut config.exe_path
        {
            exe_path.set_extension("cmd");
        }
    });

    Ok(Json(LocateExecutablesOutput {
        exes,
        globals_lookup_dirs,
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
pub fn pre_run(Json(input): Json<RunHook>) -> FnResult<Json<RunHookResult>> {
    let mut result = RunHookResult::default();

    let Some(globals_dir) = &input.globals_dir else {
        return Ok(Json(result));
    };

    let args = &input.passthrough_args;
    let config = get_tool_config::<NodeDepmanToolConfig>()?;

    if args.len() < 3 || !config.shared_globals_dir {
        return Ok(Json(result));
    }

    let env = get_host_environment()?;
    let manager = PackageManager::detect()?;

    // Includes trailing /bin folder
    let globals_bin_dir = globals_dir.real_path_string().unwrap();
    // Parent directory, doesn't include /bin folder
    let globals_root_dir = globals_dir
        .real_path()
        .unwrap()
        .parent()
        .unwrap()
        .to_string_lossy()
        .to_string();

    match manager {
        // npm install|add|etc -g <dep>
        PackageManager::Npm => {
            let has_global = args
                .iter()
                .any(|arg| arg == "--global" || arg == "-g" || arg == "--location=global");
            let has_location = args.iter().any(|arg| arg == "--location")
                && args.iter().any(|arg| arg == "global");

            if (has_global || has_location) && args.iter().all(|arg| arg != "--prefix") {
                result
                    .env
                    .get_or_insert(HashMap::default())
                    // Unix will create a /bin directory when installing into the root,
                    // while Windows installs directly into the /bin directory.
                    .insert(
                        "PREFIX".into(),
                        if env.os.is_windows() {
                            globals_bin_dir
                        } else {
                            globals_root_dir
                        },
                    );
            }
        }

        // pnpm add|update|etc -g <dep>
        PackageManager::Pnpm => {
            let aliases = [
                "add", "update", "remove", "list", "outdated", "why", "root", "bin", "env",
                "config",
            ];

            if aliases.iter().any(|alias| *alias == args[0])
                && args.iter().any(|arg| arg == "--global" || arg == "-g")
                && args
                    .iter()
                    .all(|arg| arg != "--global-dir" && arg != "--global-bin-dir")
            {
                // These arguments aren't ideal, but pnpm doesn't support
                // environment variables from what I've seen...
                let new_args = result.args.get_or_insert(vec![]);
                new_args.push("--global-dir".into());
                new_args.push(globals_root_dir);
                new_args.push("--global-bin-dir".into());
                new_args.push(globals_bin_dir);
            }
        }

        // yarn global add|remove|etc <dep>
        PackageManager::Yarn => {
            if args[0] == "global" && args.iter().all(|arg| arg != "--prefix") {
                result
                    .env
                    .get_or_insert(HashMap::default())
                    // Both Unix and Windows will create a /bin directory,
                    // when installing into the root.
                    .insert("PREFIX".into(), globals_root_dir);
            }
        }
    };

    Ok(Json(result))
}

fn create_internal_shim(
    env: &HostEnvironment,
    tool_dir: &VirtualPath,
    shim_name: &str,
    bin_file: &str,
) -> AnyResult<()> {
    if env.os.is_windows() {
        fs::write_file(
            tool_dir.join("shims").join(format!("{shim_name}.cmd")),
            CMD_SHIM_TEMPLATE.replace("{bin_path}", &format!("..\\bin\\{bin_file}")),
        )?;
    } else {
        fs::write_file(
            tool_dir.join("shims").join(shim_name),
            BASH_SHIM_TEMPLATE.replace("{bin_path}", &format!("../bin/{bin_file}")),
        )?;
    }

    Ok(())
}

fn create_internal_shims(
    env: &HostEnvironment,
    tool_dir: &VirtualPath,
    package_manager: &PackageManager,
) -> AnyResult<()> {
    match package_manager {
        PackageManager::Npm => {
            create_internal_shim(env, tool_dir, "npm", "npm-cli.js")?;
            create_internal_shim(env, tool_dir, "npx", "npx-cli.js")?;
        }
        PackageManager::Pnpm => {
            create_internal_shim(env, tool_dir, "pnpm", "pnpm.cjs")?;
            create_internal_shim(env, tool_dir, "pnpx", "pnpx.cjs")?;
        }
        PackageManager::Yarn => {
            create_internal_shim(env, tool_dir, "yarn", "yarn.js")?;
        }
    };

    Ok(())
}
