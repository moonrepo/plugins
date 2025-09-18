use crate::schema::{ExecutableSchema, PlatformMapper, Schema, SchemaType};
use extism_pdk::*;
use proto_pdk::*;
use regex::Captures;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

fn get_schema() -> Result<Schema, Error> {
    let data = config::get("proto_schema")?.expect("Missing schema!");
    let schema: Schema = json::from_str(&data)?;

    Ok(schema)
}

fn get_platform<'schema>(
    schema: &'schema Schema,
    env: &HostEnvironment,
) -> Result<&'schema PlatformMapper, PluginError> {
    let mut platform = schema.platform.get(&env.os);

    // Fallback to linux for other OSes
    if platform.is_none() && env.os.is_bsd() {
        platform = schema.platform.get(&HostOS::Linux);
    }

    platform.ok_or_else(|| PluginError::UnsupportedOS {
        tool: schema.name.clone(),
        os: env.os.to_rust_os(),
    })
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    let env = get_host_environment()?;
    let schema = get_schema()?;
    let platform = get_platform(&schema, &env)?;
    let mut deprecations = schema.deprecations.clone();

    #[allow(deprecated)]
    if platform.bin_path.is_some() {
        deprecations.push(
            format!("The <property>platform.{os}.bin-path</property> setting is deprecated, use <property>platform.{os}.exe-path</property> instead.", os = env.os)
        );
    }

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

    Ok(Json(RegisterToolOutput {
        name: schema.name,
        type_of: match schema.type_of {
            SchemaType::CommandLine => PluginType::CommandLine,
            SchemaType::DependencyManager => PluginType::DependencyManager,
            SchemaType::Language => PluginType::Language,
            SchemaType::VersionManager => PluginType::VersionManager,
        },
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        default_version: schema.metadata.default_version,
        plugin_version: match schema.metadata.plugin_version {
            Some(version) => Some(version),
            None => Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        },
        self_upgrade_commands: schema.metadata.self_upgrade_commands,
        deprecations,
        requires: schema.metadata.requires,
        ..RegisterToolOutput::default()
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
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let schema = get_schema()?;
    let mut versions: HashSet<VersionSpec> = HashSet::from_iter(schema.resolve.versions);

    // Git tags
    if let Some(repository) = schema.resolve.git_url {
        let pattern = regex::Regex::new(
            schema
                .resolve
                .git_tag_pattern
                .as_ref()
                .unwrap_or(&schema.resolve.version_pattern),
        )?;

        for tag in load_git_tags(repository)? {
            if let Some(cap) = pattern.captures(&tag) {
                versions.insert(VersionSpec::parse(create_version(cap))?);
            }
        }
    }
    // URL endpoint
    else if let Some(endpoint) = schema.resolve.manifest_url {
        let pattern = regex::Regex::new(&schema.resolve.version_pattern)?;
        let version_key = &schema.resolve.manifest_version_key;
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
    output.aliases.extend(schema.resolve.aliases);

    if output.versions.is_empty() {
        return Err(plugin_err!(
            "Unable to resolve versions for {}. Schema either requires a <property>resolve.git-url</property> or <property>resolve.manifest-url</property>.",
            schema.name
        ));
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    let schema = get_schema()?;

    Ok(Json(DetectVersionOutput {
        files: schema.detect.version_files,
        ignore: schema.detect.ignore,
    }))
}

fn interpolate_tokens(
    value: &str,
    version: &VersionSpec,
    schema: &Schema,
    env: &HostEnvironment,
) -> String {
    let arch = env.arch.to_rust_arch();
    let os = env.os.to_string();

    let mut value = value
        .replace("{version}", &version.to_string())
        .replace(
            "{arch}",
            schema.install.arch.get(&env.arch).unwrap_or(&arch),
        )
        .replace("{os}", &os);

    // Avoid detecting musl unless requested
    if value.contains("{libc}") {
        let libc = env.libc.to_string();

        value = value.replace(
            "{libc}",
            schema.install.libc.get(&env.libc).unwrap_or(&libc),
        );
    }

    if let Some(v) = version.as_version() {
        let major = v.major.to_string();
        let minor = v.minor.to_string();
        let patch = v.patch.to_string();
        let year = format!("{:0>4}", v.major);
        let month = format!("{:0>2}", v.minor);
        let day = format!("{:0>2}", v.patch);
        let major_minor = format!("{}.{}", v.major, v.minor); // Deprecated, remains for backwards compatibility
        let year_month = format!("{:0>4}-{:0>2}", v.major, v.minor); // Deprecated, remains for backwards compatibility
        let pre = v.pre.to_string();
        let build = v.build.to_string();

        value = value
            .replace("{versionMajor}", &major)
            .replace("{versionMinor}", &minor)
            .replace("{versionPatch}", &patch)
            .replace("{versionMajorMinor}", &major_minor) // Deprecated, remains for backwards compatibility
            .replace("{versionYear}", &year)
            .replace("{versionMonth}", &month)
            .replace("{versionDay}", &day)
            .replace("{versionYearMonth}", &year_month) // Deprecated, remains for backwards compatibility
            .replace("{versionPrerelease}", &pre)
            .replace("{versionBuild}", &build);
    } else {
        value = value
            .replace("{versionMajor}", "")
            .replace("{versionMinor}", "")
            .replace("{versionPatch}", "")
            .replace("{versionMajorMinor}", "") // Deprecated, remains for backwards compatibility
            .replace("{versionYear}", "")
            .replace("{versionMonth}", "")
            .replace("{versionDay}", "")
            .replace("{versionYearMonth}", "") // Deprecated, remains for backwards compatibility
            .replace("{versionPrerelease}", "")
            .replace("{versionBuild}", "");
    }

    value
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let schema = get_schema()?;
    let platform = get_platform(&schema, &env)?;

    if !platform.archs.is_empty() {
        check_supported_os_and_arch(
            &schema.name,
            &env,
            HashMap::from_iter([(env.os, platform.archs.clone())]),
        )?;
    }

    let version = &input.context.version;
    let is_canary = version.is_canary();

    let download_file = interpolate_tokens(&platform.download_file, version, &schema, &env);

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
        version,
        &schema,
        &env,
    )
    .replace("{download_file}", &download_file);

    let checksum_file = interpolate_tokens(
        platform.checksum_file.as_deref().unwrap_or("CHECKSUM.txt"),
        version,
        &schema,
        &env,
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
        interpolate_tokens(url, version, &schema, &env).replace("{checksum_file}", &checksum_file)
    });

    let archive_prefix = platform
        .archive_prefix
        .as_ref()
        .map(|prefix| interpolate_tokens(prefix, version, &schema, &env));

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix,
        checksum_url,
        checksum_name: Some(checksum_file),
        checksum_public_key: schema.install.checksum_public_key,
        download_url,
        download_name: Some(download_file),
        ..DownloadPrebuiltOutput::default()
    }))
}

fn create_executable_config(schema: ExecutableSchema) -> ExecutableConfig {
    ExecutableConfig {
        exe_path: schema.exe_path,
        exe_link_path: schema.exe_link_path,
        no_bin: schema.no_bin,
        no_shim: schema.no_shim,
        parent_exe_args: schema.parent_exe_args,
        parent_exe_name: schema.parent_exe_name,
        shim_before_args: schema.shim_before_args.map(StringOrVec::Vec),
        shim_after_args: schema.shim_after_args.map(StringOrVec::Vec),
        shim_env_vars: schema.shim_env_vars.map(HashMap::from_iter),
        ..Default::default()
    }
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let schema = get_schema()?;
    let platform = get_platform(&schema, &env)?;
    let id = get_plugin_id()?;

    // On Windows, automatically add the `.exe` extension to all executables.
    // But only if there is no extension, so that we don't overwrite `.js` and others!
    let append_exe_ext = |mut path: PathBuf| -> PathBuf {
        if env.os.is_windows() && path.extension().is_none() {
            path.set_extension("exe");
        }

        path
    };

    let prepare_primary_exe = |config: &mut ExecutableConfig| {
        config.primary = true;

        #[allow(deprecated)]
        let exe_path = append_exe_ext(
            // Name from platform
            platform
                .exe_path
                .as_ref()
                .or(platform.bin_path.as_ref())
                // Name from config
                .or(config.exe_path.as_ref())
                // Name from plugin ID
                .map_or_else(|| PathBuf::from(id.as_str()), |path| path.to_owned()),
        );

        config.exe_path = Some(
            interpolate_tokens(
                exe_path.to_str().unwrap_or("<invalidpath>"),
                &input.context.version,
                &schema,
                &env,
            )
            .into(),
        );

        if let Some(no_bin) = schema.install.no_bin {
            config.no_bin = no_bin;
        }

        if let Some(no_shim) = schema.install.no_shim {
            config.no_shim = no_shim;
        }
    };

    let prepare_secondary_exe = |config: &mut ExecutableConfig| {
        if let Some(exe_path) = config.exe_path.take() {
            config.exe_path = Some(append_exe_ext(exe_path));
        }

        if let Some(exe_link_path) = config.exe_link_path.take() {
            config.exe_link_path = Some(append_exe_ext(exe_link_path));
        }
    };

    // Executables
    let mut has_primary = false;
    let mut exes = schema
        .install
        .exes
        .iter()
        .map(|(key, value)| {
            let mut config = create_executable_config(value.to_owned());

            if config.primary {
                has_primary = true;
                prepare_primary_exe(&mut config);
            } else {
                prepare_secondary_exe(&mut config);
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
            .map(create_executable_config)
            .unwrap_or_default();

        prepare_primary_exe(&mut primary);

        exes.insert(id.to_string(), primary.clone());
    }

    #[allow(deprecated)]
    schema.install.secondary.iter().for_each(|(key, value)| {
        exes.entry(key.to_owned()).or_insert_with(|| {
            let mut config = create_executable_config(value.to_owned());

            prepare_secondary_exe(&mut config);

            config
        });
    });

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter(exes),
        exes_dirs: if platform.exes_dirs.is_empty() {
            #[allow(deprecated)]
            match platform.exes_dir.as_ref() {
                Some(dir) => vec![dir.into()],
                None => vec![],
            }
        } else {
            platform.exes_dirs.clone()
        },
        globals_lookup_dirs: schema.packages.globals_lookup_dirs,
        globals_prefix: schema.packages.globals_prefix,
        ..Default::default()
    }))
}
