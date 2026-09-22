use crate::schema::{Schema, interpolate_tokens, v1::SchemaType};
use extism_pdk::*;
use proto_pdk::*;
use regex::Captures;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tool_common::enable_tracing;

macro_rules! override_option {
    ($prev:ident, $next:ident, [ $($prop:ident),* ]) => {
        $(
            override_option!($prev, $next, $prop);
        )*
    };
    ($prev:ident, $next:ident, $prop:ident) => {
        if let Some(value) = &$next.$prop {
            $prev.$prop = Some(value.to_owned());
        }
    };
}

macro_rules! override_value {
    ($prev:ident, $next:ident, [ $($prop:ident),* ]) => {
        $(
            override_value!($prev, $next, $prop);
        )*
    };
    ($prev:ident, $next:ident, $prop:ident) => {
        if !$next.$prop.is_empty() {
            $prev.$prop = $next.$prop.clone();
        }
    };
}

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

fn get_schema() -> Result<Schema, Error> {
    let data = config::get("proto_schema")?.expect("Missing schema!");
    let value: JsonValue = json::from_str(&data)?;

    let schema = if value.get("format").is_some_and(|format| {
        format.is_string() && format == "2" || format.is_number() && format == 2
    }) {
        Schema::V2(json::from_value(value)?)
    } else {
        Schema::V1(json::from_value(value)?)
    };

    Ok(schema)
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    let schema = get_schema()?;

    Ok(Json(match schema {
        Schema::V1(schema) => {
            let mut deprecations = schema.deprecations.clone();

            #[allow(deprecated)]
            if schema.install.primary.is_some() {
                deprecations.push(
                    "The <property>install.primary</property> setting is deprecated, use <property>install.exes</property> and the <symbol>primary</symbol> flag instead.".into()
                );
            }

            #[allow(deprecated)]
            if !schema.install.secondary.is_empty() {
                deprecations.push(
                    "The <property>install.secondary</property> setting is deprecated, use <property>install.exes</property> instead.".into()
                );
            }

            RegisterToolOutput {
                name: schema.name,
                type_of: match schema.type_of {
                    SchemaType::CommandLine => PluginType::CommandLine,
                    SchemaType::DependencyManager => PluginType::DependencyManager,
                    SchemaType::Language => PluginType::Language,
                    SchemaType::VersionManager => PluginType::VersionManager,
                },
                minimum_proto_version: Some(Version::new(0, 60, 0)),
                default_version: schema.metadata.default_version,
                plugin_version: match schema.metadata.plugin_version {
                    Some(version) => Some(version),
                    None => Version::parse(env!("CARGO_PKG_VERSION")).ok(),
                },
                self_upgrade_commands: schema.metadata.self_upgrade_commands,
                deprecations,
                requires: schema.metadata.requires,
                ..Default::default()
            }
        }

        Schema::V2(schema) => RegisterToolOutput {
            minimum_proto_version: schema
                .metadata
                .minimum_proto_version
                .or_else(|| Some(Version::new(0, 60, 0))),
            plugin_version: schema
                .metadata
                .plugin_version
                .or_else(|| Version::parse(env!("CARGO_PKG_VERSION")).ok()),
            ..schema.metadata
        },
    }))
}

fn create_version(cap: Captures) -> String {
    // If no named, use entire string (legacy)
    if cap.name("major").is_none() {
        return cap.get(1).unwrap().as_str().to_string();
    }

    // Otherwise piece named parts together
    let mut version = String::new();

    version.push_str(
        cap.name("major")
            .or_else(|| cap.name("year"))
            .map(|c| c.as_str())
            .unwrap_or("0"),
    );
    version.push('.');
    version.push_str(
        cap.name("minor")
            .or_else(|| cap.name("month"))
            .map(|c| c.as_str())
            .unwrap_or("0"),
    );
    version.push('.');
    version.push_str(
        cap.name("patch")
            .or_else(|| cap.name("day"))
            .map(|c| c.as_str())
            .unwrap_or("0"),
    );

    if let Some(pre) = cap.name("pre").map(|c| c.as_str()) {
        if !pre.starts_with('-') {
            version.push('-');
        }
        version.push_str(pre);
    }

    if let Some(build) = cap.name("build").map(|c| c.as_str()) {
        if !build.starts_with('+') {
            version.push('+');
        }
        version.push_str(build);
    }

    version
}

#[plugin_fn]
pub fn detect_version_files(
    Json(_): Json<DetectVersionInput>,
) -> FnResult<Json<DetectVersionOutput>> {
    let schema = get_schema()?;

    Ok(Json(match schema {
        Schema::V1(schema) => DetectVersionOutput {
            files: schema.detect.version_files,
            ignore: schema.detect.ignore,
        },
        Schema::V2(schema) => schema.detect,
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let schema = get_schema()?;
    let mut versions: HashSet<VersionSpec> = HashSet::from_iter(schema.source_versions());
    let aliases = schema.source_aliases();

    // Git tags
    if let Some(repository) = schema.resolve_git_url() {
        let pattern = regex::Regex::new(schema.resolve_git_tag_pattern())?;

        for tag in load_git_tags(repository)? {
            if let Some(cap) = pattern.captures(&tag) {
                versions.insert(VersionSpec::parse(create_version(cap))?);
            }
        }
    }
    // URL endpoint
    else if let Some(endpoint) = schema.resolve_manifest_url() {
        let pattern = regex::Regex::new(schema.resolve_manifest_version_pattern())?;
        let version_key = schema.resolve_manifest_version_key();
        let response: Vec<JsonValue> = fetch_json(endpoint)?;

        for row in response {
            match row {
                JsonValue::String(v) => {
                    if let Some(cap) = pattern.captures(&v) {
                        versions.insert(VersionSpec::parse(create_version(cap))?);
                    }
                }
                JsonValue::Object(o) => {
                    if let Some(JsonValue::String(v)) = o.get(version_key)
                        && let Some(cap) = pattern.captures(v)
                    {
                        versions.insert(VersionSpec::parse(create_version(cap))?);
                    }
                }
                _ => {}
            }
        }
    }

    let mut output = LoadVersionsOutput::from_versions(versions.into_iter().collect());
    output.aliases.extend(aliases);

    if output.versions.is_empty() {
        return Err(plugin_err!(
            "Unable to resolve versions for {}. Schema requires either a Git repository or registry index URL.",
            schema.get_name()
        ));
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let schema = get_schema()?;
    let platform = schema.get_platform(env, Some(&input.context.version))?;

    if !platform.archs.is_empty() {
        check_supported_os_and_arch(
            schema.get_name(),
            env,
            HashMap::from_iter([(env.os, platform.archs.clone())]),
        )?;
    }

    let spec = &input.context.version;
    let is_canary = spec.is_canary();

    let output: DownloadPrebuiltOutput = match schema {
        Schema::V1(schema) => {
            let download_file = interpolate_tokens(
                &platform.download_name.clone().unwrap_or_default(),
                env,
                spec,
                &platform,
            );

            let download_url = interpolate_tokens(
                if is_canary {
                    schema
                        .install
                        .download_url_canary
                        .as_ref()
                        .unwrap_or(&schema.install.download_url)
                } else {
                    &schema.install.download_url
                },
                env,
                spec,
                &platform,
            )
            .replace("{download_file}", &download_file);

            let checksum_file = interpolate_tokens(
                platform.checksum_name.as_deref().unwrap_or("CHECKSUM.txt"),
                env,
                spec,
                &platform,
            );

            let checksum_url = if is_canary {
                schema
                    .install
                    .checksum_url_canary
                    .as_ref()
                    .or(schema.install.checksum_url.as_ref())
            } else {
                schema.install.checksum_url.as_ref()
            };

            let checksum_url = checksum_url.map(|url| {
                interpolate_tokens(url, env, spec, &platform)
                    .replace("{checksum_file}", &checksum_file)
            });

            let archive_prefix = platform
                .archive_prefix
                .as_ref()
                .map(|prefix| interpolate_tokens(prefix, env, spec, &platform));

            DownloadPrebuiltOutput {
                archive_prefix,
                checksum_url,
                checksum_name: Some(checksum_file),
                checksum_public_key: schema.install.checksum_public_key,
                download_url,
                download_name: Some(download_file),
                ..Default::default()
            }
        }
        Schema::V2(schema) => {
            let output =
                schema.apply_layers(env, spec, |prev: &mut DownloadPrebuiltOutput, layer| {
                    if let Some(next) = layer.install {
                        override_option!(
                            prev,
                            next,
                            [
                                archive_prefix,
                                checksum,
                                checksum_name,
                                checksum_public_key,
                                checksum_url,
                                download_name,
                                post_script
                            ]
                        );

                        override_value!(prev, next, [download_url, http_headers, post_script_args]);
                    }

                    // Platform settings beat install settings within the same layer
                    if let Some(next) = layer.platform {
                        override_option!(
                            prev,
                            next,
                            [archive_prefix, checksum_name, download_name]
                        );
                    }
                })?;

            let archive_prefix = output
                .archive_prefix
                .map(|prefix| interpolate_tokens(&prefix, env, spec, &platform));

            let download_name = output
                .download_name
                .map(|name| interpolate_tokens(&name, env, spec, &platform));

            let download_url = interpolate_tokens(&output.download_url, env, spec, &platform)
                .replace(
                    "{download_file}",
                    download_name.as_deref().unwrap_or_default(),
                )
                .replace(
                    "{download_name}",
                    download_name.as_deref().unwrap_or_default(),
                );

            let checksum_name = output
                .checksum_name
                .map(|name| interpolate_tokens(&name, env, spec, &platform));

            let checksum_url = output.checksum_url.map(|url| {
                interpolate_tokens(&url, env, spec, &platform)
                    .replace(
                        "{checksum_file}",
                        checksum_name.as_deref().unwrap_or_default(),
                    )
                    .replace(
                        "{checksum_name}",
                        checksum_name.as_deref().unwrap_or_default(),
                    )
            });

            DownloadPrebuiltOutput {
                archive_prefix,
                checksum_name,
                checksum_url,
                download_name,
                download_url,
                ..output
            }
        }
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let id = get_plugin_id()?;
    let env = get_host_environment()?;
    let schema = get_schema()?;
    let platform = schema.get_platform(env, Some(&input.context.version))?;

    let prepare_exe_path = |mut path: PathBuf| -> PathBuf {
        // On Windows, automatically add the `.exe` extension to all executables.
        // But only if there is no extension, so that we don't overwrite `.js` and others!
        if env.os.is_windows() && path.extension().is_none() {
            path.set_extension("exe");
        }

        // If we can convert the path into a string, we should interpolate
        // tokens, otherwise return it as-is
        if let Some(inner) = path.to_str() {
            return interpolate_tokens(inner, env, &input.context.version, &platform).into();
        }

        path
    };

    let prepare_exe_config = |config: &mut ExecutableConfig| {
        if let Some(exe_path) = config.exe_path.take() {
            config.exe_path = Some(prepare_exe_path(exe_path));
        }

        if let Some(exe_link_path) = config.exe_link_path.take() {
            config.exe_link_path = Some(prepare_exe_path(exe_link_path));
        }
    };

    let output: LocateExecutablesOutput = match schema {
        Schema::V1(schema) => {
            let prepare_primary_exe_config = |config: &mut ExecutableConfig| {
                config.primary = true;

                #[allow(deprecated)]
                let exe_path =
                    // Name from platform
                    platform
                    .exe_path.as_ref()
                    // Name from config
                    .or(config.exe_path.as_ref())
                    // Name from plugin ID
                    .map_or_else(|| PathBuf::from(id.as_str()), |path| path.to_owned());

                config.exe_path = Some(exe_path);

                if let Some(no_bin) = schema.install.no_bin {
                    config.no_bin = no_bin;
                }

                if let Some(no_shim) = schema.install.no_shim {
                    config.no_shim = no_shim;
                }

                prepare_exe_config(config);
            };

            // Executables
            let mut has_primary = false;
            let mut exes = schema
                .install
                .exes
                .iter()
                .map(|(key, value)| {
                    let mut config = value.to_owned().into_config();

                    if config.primary {
                        has_primary = true;
                        prepare_primary_exe_config(&mut config);
                    } else {
                        prepare_exe_config(&mut config);
                    }

                    (key.to_string(), config)
                })
                .collect::<HashMap<_, _>>();

            // Primary & secondary exe's (deprecated)
            if !has_primary {
                #[allow(deprecated)]
                let mut primary = schema
                    .install
                    .primary
                    .clone()
                    .map(|exe| exe.into_config())
                    .unwrap_or_default();

                prepare_primary_exe_config(&mut primary);

                exes.insert(id.to_string(), primary.clone());
            }

            #[allow(deprecated)]
            schema.install.secondary.iter().for_each(|(key, value)| {
                exes.entry(key.to_owned()).or_insert_with(|| {
                    let mut config = value.to_owned().into_config();

                    prepare_exe_config(&mut config);

                    config
                });
            });

            LocateExecutablesOutput {
                exes: HashMap::from_iter(exes),
                exes_dirs: platform.exes_dirs.unwrap_or_default(),
                globals_lookup_dirs: schema.packages.globals_lookup_dirs,
                globals_prefix: schema.packages.globals_prefix,
            }
        }

        Schema::V2(schema) => {
            let mut output = schema.apply_layers(
                env,
                &input.context.version,
                |prev: &mut LocateExecutablesOutput, layer| {
                    if let Some(next) = layer.locate {
                        override_option!(prev, next, globals_prefix);
                        override_value!(prev, next, [exes, exes_dirs, globals_lookup_dirs]);
                    }

                    // Platform settings beat locate settings within the same layer
                    if let Some(next) = layer.platform {
                        if let Some(exe_path) = &next.exe_path {
                            for config in prev.exes.values_mut().filter(|config| config.primary) {
                                config.exe_path = Some(exe_path.to_owned());
                            }
                        }

                        if let Some(dirs) = &next.exes_dirs
                            && !dirs.is_empty()
                        {
                            prev.exes_dirs = dirs.to_owned();
                        }
                    }
                },
            )?;

            for config in output.exes.values_mut() {
                prepare_exe_config(config);
            }

            output
        }
    };

    Ok(Json(output))
}
