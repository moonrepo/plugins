use crate::config::NodeDepmanPluginConfig;
use crate::npm_registry::parse_registry_response;
use crate::package_manager::PackageManager;
use extism_pdk::*;
use lang_javascript_common::{
    NodeDistVersion, extract_engine_version, extract_package_manager_version, extract_volta_version,
};
use nodejs_package_json::PackageJson;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn get_env_var(key: &str) -> String;
    fn to_virtual_path(input: String) -> String;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    let manager = PackageManager::detect()?;

    Ok(Json(RegisterToolOutput {
        name: manager.to_string(),
        type_of: PluginType::DependencyManager,
        config_schema: Some(SchemaBuilder::build_root::<NodeDepmanPluginConfig>()),
        default_version: if manager == PackageManager::Npm {
            Some(UnresolvedVersionSpec::Alias("bundled".into()))
        } else {
            None
        },
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        requires: vec!["node".into()],
        ..RegisterToolOutput::default()
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

    if input.file == "package.json" {
        if let Ok(package_json) = json::from_str::<PackageJson>(&input.content) {
            let manager_name = PackageManager::detect()?.to_string();

            if let Some(constraint) = extract_package_manager_version(&package_json, &manager_name)
            {
                version = Some(UnresolvedVersionSpec::parse(constraint)?);
            }

            if version.is_none() {
                if let Some(constraint) =
                    extract_volta_version(&package_json, &input.path, &manager_name)?
                {
                    version = Some(UnresolvedVersionSpec::parse(constraint)?);
                }
            }

            if version.is_none() {
                if let Some(constraint) = extract_engine_version(&package_json, &manager_name) {
                    version = Some(UnresolvedVersionSpec::parse(constraint)?);
                }
            }
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let manager = PackageManager::detect()?;
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
        map_output(fetch_text("https://registry.npmjs.org/yarn/")?, true)?;
        map_output(
            fetch_text("https://registry.npmjs.org/@yarnpkg/cli-dist/")?,
            true,
        )?;
    } else {
        map_output(
            fetch_text(format!("https://registry.npmjs.org/{package_name}/"))?,
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
    if manager.is_yarn_classic(spec.to_unresolved_spec()) {
        if let Some(version) = spec.as_version() {
            // Prefix changed to "package" in v1.22.20
            // https://github.com/yarnpkg/yarn/releases/tag/v1.22.20
            if version.minor <= 22 && version.patch <= 19 {
                return format!("yarn-v{version}");
            }
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

    let host = get_tool_config::<NodeDepmanPluginConfig>()?.dist_url;
    let filename = format!("{package_without_scope}-{version}.tgz");

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(get_archive_prefix(&manager, version)),
        download_url: host
            .replace("{package}", &package_name)
            .replace("{package_without_scope}", package_without_scope)
            .replace("{version}", &version.to_string())
            .replace("{file}", &filename),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let manager = PackageManager::detect()?;
    let mut secondary = HashMap::<String, ExecutableConfig>::default();
    let mut primary;

    // These are the directories that contain the executable binaries,
    // NOT where the packages/node modules are stored. Some package managers
    // have separate folders for the 2 processes, and then create symlinks.
    let mut globals_lookup_dirs = vec!["$PREFIX/bin".into()];

    // We don't link binaries for package managers for the following reasons:
    // 1 - We can't link JS files because they aren't executable.
    // 2 - We can't link the bash/cmd wrappers, as they expect the files to exist
    //     relative from the node install directory, which they do not.
    match &manager {
        PackageManager::Npm => {
            primary = ExecutableConfig::with_parent("bin/npm-cli.js", "node");
            primary.primary = true;
            primary.no_bin = true;

            // npx
            let mut npx = ExecutableConfig::with_parent("bin/npx-cli.js", "node");
            npx.no_bin = true;

            secondary.insert("npx".into(), npx);

            // node-gyp
            let mut node_gyp =
                ExecutableConfig::with_parent("node_modules/node-gyp/bin/node-gyp.js", "node");
            node_gyp.no_bin = true;

            secondary.insert("node-gyp".into(), node_gyp);

            // https://docs.npmjs.com/cli/v9/configuring-npm/folders#prefix-configuration
            // https://github.com/npm/cli/blob/latest/lib/npm.js
            // https://github.com/npm/cli/blob/latest/workspaces/config/lib/index.js#L339
            globals_lookup_dirs.push("$TOOL_DIR/bin".into());
        }
        PackageManager::Pnpm => {
            primary = ExecutableConfig::with_parent("bin/pnpm.cjs", "node");
            primary.primary = true;
            primary.no_bin = true;

            // pnpx
            secondary.insert(
                "pnpx".into(),
                ExecutableConfig {
                    no_bin: true,
                    shim_before_args: Some(StringOrVec::String("dlx".into())),
                    ..ExecutableConfig::default()
                },
            );

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
            primary = ExecutableConfig::with_parent("bin/yarn.js", "node");
            primary.primary = true;
            primary.no_bin = true;

            // yarnpkg
            secondary.insert(
                "yarnpkg".into(),
                ExecutableConfig {
                    primary: false,
                    ..primary.clone()
                },
            );

            // https://github.com/yarnpkg/yarn/blob/master/src/cli/commands/global.js#L84
            if env.os.is_windows() {
                globals_lookup_dirs.push("$LOCALAPPDATA\\Yarn\\bin".into());
                globals_lookup_dirs.push("$HOME\\.yarn\\bin".into());
            } else {
                globals_lookup_dirs.push("$HOME/.yarn/bin".into());
            }
        }
    };

    let config = get_tool_config::<NodeDepmanPluginConfig>()?;

    if config.shared_globals_dir {
        globals_lookup_dirs.clear();
        globals_lookup_dirs.push("$PROTO_HOME/tools/node/globals/bin".into());
    }

    let mut exes = HashMap::from_iter([(manager.to_string(), primary)]);
    exes.extend(secondary);

    Ok(Json(LocateExecutablesOutput {
        exes,
        exes_dirs: vec![".".into()],
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
    let config = get_tool_config::<NodeDepmanPluginConfig>()?;

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
