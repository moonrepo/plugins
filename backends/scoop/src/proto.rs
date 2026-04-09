use crate::config::ScoopToolConfig;
use crate::manifest::ScoopManifest;
use backend_common::enable_tracing;
use extism_pdk::*;
use proto_pdk::*;
use rustc_hash::FxHashMap;
use schematic::SchemaBuilder;

fn is_scoop() -> bool {
    get_plugin_id().is_ok_and(|id| id == "scoop")
}

fn map_arch(arch: HostArch) -> &'static str {
    match arch {
        HostArch::X86 => "32bit",
        HostArch::X64 => "64bit",
        HostArch::Arm64 => "arm64",
        _ => "64bit",
    }
}

fn fetch_manifest(config: &ScoopToolConfig) -> AnyResult<ScoopManifest> {
    let url = config.get_manifest_url()?;
    let manifest: ScoopManifest = fetch_json(&url)?;

    Ok(manifest)
}

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: if input.id == "scoop" {
            input.id.to_string()
        } else {
            format!("scoop:{}", input.id)
        },
        type_of: if input.id == "scoop" {
            PluginType::VersionManager
        } else {
            PluginType::Language
        },
        minimum_proto_version: Some(Version::new(0, 56, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Toggle(true),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<ScoopToolConfig>(),
    }))
}

#[plugin_fn]
pub fn register_backend(
    Json(input): Json<RegisterBackendInput>,
) -> FnResult<Json<RegisterBackendOutput>> {
    let env = get_host_environment()?;

    if !env.os.is_windows() {
        return Err(PluginError::UnsupportedOS {
            tool: input.id.to_string(),
            os: env.os.to_string(),
        }
        .into());
    }

    let config = get_tool_config::<ScoopToolConfig>()?;

    Ok(Json(RegisterBackendOutput {
        backend_id: Id::new(config.get_manifest_name()?)?,
        exes: vec![],
        source: None,
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();

    if is_scoop() {
        return Ok(Json(output));
    }

    let config = get_tool_config::<ScoopToolConfig>()?;
    let manifest = fetch_manifest(&config)?;

    let version = VersionSpec::parse(&manifest.version)?;
    let unresolved = UnresolvedVersionSpec::parse(&manifest.version)?;

    output.versions.push(version);
    output.latest = Some(unresolved.clone());
    output.aliases.insert("latest".into(), unresolved.clone());
    output.aliases.insert("stable".into(), unresolved);

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    if let UnresolvedVersionSpec::Alias(alias) = &input.initial
        && (alias == "latest" || alias == "stable")
    {
        let config = get_tool_config::<ScoopToolConfig>()?;
        let manifest = fetch_manifest(&config)?;

        output.candidate = Some(UnresolvedVersionSpec::parse(&manifest.version)?);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        "Scoop",
        &env,
        permutations![
            HostOS::Windows => [HostArch::X64, HostArch::X86, HostArch::Arm64],
        ],
    )?;

    let config = get_tool_config::<ScoopToolConfig>()?;
    let manifest = fetch_manifest(&config)?;
    let arch = map_arch(env.arch);
    let version = input.context.version.to_string();
    let is_current = version == manifest.version;

    let (download_url, archive_prefix, checksum) = if is_current {
        // Use direct URLs from the manifest for the current version
        let url = manifest.get_url_for_arch(arch).ok_or_else(|| {
            PluginError::Message(format!(
                "No download URL found for architecture {arch} in the Scoop manifest"
            ))
        })?;
        let extract_dir = manifest.get_extract_dir_for_arch(arch);
        let hash = manifest.get_hash_for_arch(arch);

        (url, extract_dir, hash)
    } else {
        // Use autoupdate URL templates for other versions
        let url = manifest
            .resolve_autoupdate_url(&version, arch)
            .ok_or_else(|| {
                PluginError::Message(format!(
                    "No autoupdate URL template found for architecture {arch} in the Scoop manifest. \
                     Only the current version ({}) can be downloaded without autoupdate configuration.",
                    manifest.version
                ))
            })?;
        let extract_dir = manifest.resolve_autoupdate_extract_dir(&version, arch);

        // Checksums are not available for non-current versions via simple autoupdate
        (url, extract_dir, None)
    };

    Ok(Json(DownloadPrebuiltOutput {
        download_url,
        archive_prefix,
        checksum: checksum.map(Checksum::sha256),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let mut output = LocateExecutablesOutput::default();

    if is_scoop() {
        return Ok(Json(output));
    }

    let env = get_host_environment()?;
    let arch = map_arch(env.arch);
    let config = get_tool_config::<ScoopToolConfig>()?;
    let manifest = fetch_manifest(&config)?;

    let id = get_plugin_id()?;
    let executables = manifest.get_executables_for_arch(arch);

    for entry in &executables {
        let name = entry.alias.as_deref().unwrap_or_else(|| {
            // Extract filename without extension as the name
            entry
                .exe_path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&entry.exe_path)
        });

        let name_without_ext = name
            .strip_suffix(".exe")
            .or_else(|| name.strip_suffix(".cmd"))
            .or_else(|| name.strip_suffix(".bat"))
            .unwrap_or(name);

        output.exes.insert(
            name_without_ext.to_owned(),
            ExecutableConfig {
                primary: id.as_str() == name_without_ext,
                exe_path: Some(entry.exe_path.clone().into()),
                ..Default::default()
            },
        );
    }

    // Add env_add_path directories
    let paths = manifest.get_env_add_paths_for_arch(arch);
    for path in paths {
        if path == "." {
            output.exes_dirs.push(".".into());
        } else {
            output.exes_dirs.push(path.into());
        }
    }

    // If no executables found, add a default based on the tool ID
    if output.exes.is_empty() {
        output.exes.insert(
            id.to_string(),
            ExecutableConfig::new_primary(format!("{id}.exe")),
        );
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn pre_run(Json(_): Json<RunHook>) -> FnResult<Json<RunHookResult>> {
    let mut output = RunHookResult::default();

    let config = get_tool_config::<ScoopToolConfig>()?;
    let manifest = fetch_manifest(&config)?;

    if let Some(env_set) = manifest.env_set {
        let mut env = FxHashMap::default();
        for (key, value) in env_set {
            env.insert(key, value);
        }
        output.env = Some(env);
    }

    Ok(Json(output))
}
