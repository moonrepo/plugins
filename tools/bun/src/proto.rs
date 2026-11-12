use crate::config::BunToolConfig;
use extism_pdk::*;
use lang_javascript_common::{
    extract_dev_engine_package_manager_version, extract_dev_engine_runtime_version,
    extract_engine_version, extract_package_manager_version, extract_volta_version,
    get_dev_engine_version_error, insert_dev_engine_version, remove_dev_engine,
};
use nodejs_package_json::PackageJson;
use proto_pdk::*;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use std::collections::HashMap;
use tool_common::{append_env_bin_dir, enable_tracing};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "Bun";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 64, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        self_upgrade_commands: vec!["upgrade".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<BunToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![
            ".bumrc".into(),
            ".bun-version".into(),
            "package.json".into(),
        ],
        ignore: vec!["node_modules".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let content = fs::read_file(&input.path)?;
    let mut version = None;

    if input.file == "package.json" {
        if let Ok(package_json) = json::from_str::<PackageJson>(&content) {
            if let Some(constraint) = extract_dev_engine_runtime_version(&package_json, "bun") {
                version = Some(UnresolvedVersionSpec::parse(constraint)?);
            } else if let Some(constraint) =
                extract_dev_engine_package_manager_version(&package_json, "bun")
            {
                version = Some(UnresolvedVersionSpec::parse(constraint)?);
            }

            if version.is_none()
                && let Some(constraint) = extract_volta_version(&package_json, &input.path, "bun")?
            {
                version = Some(UnresolvedVersionSpec::parse(constraint)?);
            }

            // Like setup-bun, an exact `packageManager` takes precedence over a range
            if version.is_none()
                && let Some(constraint) = extract_package_manager_version(&package_json, "bun")
            {
                version = Some(UnresolvedVersionSpec::parse(constraint)?);
            }

            if version.is_none()
                && let Some(constraint) = extract_engine_version(&package_json, "bun")
            {
                version = Some(UnresolvedVersionSpec::parse(constraint)?);
            }
        }
    } else if let Some(constraint) = extract_version_from_text(&content) {
        version = Some(UnresolvedVersionSpec::parse(constraint)?);
    }

    Ok(Json(ParseVersionFileOutput {
        version,
        ..Default::default()
    }))
}

// Version files are detected before `package.json`, so must be updated when they exist
static VERSION_FILES: [&str; 2] = [".bumrc", ".bun-version"];

#[plugin_fn]
pub fn pin_version(Json(input): Json<PinVersionInput>) -> FnResult<Json<PinVersionOutput>> {
    let mut output = PinVersionOutput::default();

    for name in VERSION_FILES {
        let file = input.dir.join(name);

        if file.exists() {
            fs::write_file(&file, format!("{}\n", input.version))?;

            output.pinned = true;
            output.file = Some(file);

            return Ok(Json(output));
        }
    }

    let file = input.dir.join("package.json");

    if file.exists() {
        if let Some(error) = get_dev_engine_version_error(&input.version) {
            output.error = Some(error);

            return Ok(Json(output));
        }

        let mut package_json: json::Value = starbase_utils::json::read_file(&file)?;

        insert_dev_engine_version(
            &mut package_json,
            "runtime".into(),
            "bun".into(),
            input.version.to_string(),
        )?;

        starbase_utils::json::write_file_with_config(&file, &package_json, true)?;

        output.pinned = true;
        output.file = Some(file);
    } else {
        output.error = Some("No <file>package.json</file> exists in the target directory.".into());
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn unpin_version(Json(input): Json<UnpinVersionInput>) -> FnResult<Json<UnpinVersionOutput>> {
    let mut output = UnpinVersionOutput::default();

    for name in VERSION_FILES {
        let file = input.dir.join(name);

        if file.exists() {
            if let Some(version) = extract_version_from_text(&fs::read_file(&file)?) {
                output.version = Some(UnresolvedVersionSpec::parse(version)?);
            }

            fs::remove_file(&file)?;

            output.unpinned = true;
            output.file = Some(file);

            return Ok(Json(output));
        }
    }

    let file = input.dir.join("package.json");

    if file.exists() {
        let mut package_json: json::Value = starbase_utils::json::read_file(&file)?;

        if let Some(version) = remove_dev_engine(&mut package_json, "runtime".into(), "bun".into())?
        {
            output.unpinned = true;
            output.version = Some(UnresolvedVersionSpec::parse(&version)?);
            output.file = Some(file.clone());
        }

        if let Some(version) =
            remove_dev_engine(&mut package_json, "packageManager".into(), "bun".into())?
        {
            output.unpinned = true;
            output.file = Some(file.clone());

            if output.version.is_none() {
                output.version = Some(UnresolvedVersionSpec::parse(&version)?);
            }
        }

        starbase_utils::json::write_file_with_config(&file, &package_json, true)?;
    } else {
        output.error = Some("No <file>package.json</file> exists in the target directory.".into());
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/oven-sh/bun")?
        .into_iter()
        .filter_map(|tag| tag.strip_prefix("bun-v").map(|tag| tag.to_owned()))
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let version = &input.context.version;

    // Windows x64 was added in v1.1, and arm64 in v1.3.10
    let windows_arches = match version {
        VersionSpec::Version(version) => {
            if *version >= Version::new(1, 3, 10) {
                vec![HostArch::X64, HostArch::Arm64]
            } else if *version >= Version::new(1, 1, 0) {
                vec![HostArch::X64]
            } else {
                vec![]
            }
        }
        VersionSpec::Alias(alias) if alias != "latest" => vec![],
        _ => vec![HostArch::X64, HostArch::Arm64],
    };

    let mut targets = permutations! [
        HostOS::Linux => [HostArch::X64, HostArch::Arm64],
        HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
    ];

    if !windows_arches.is_empty() {
        targets.insert(HostOS::Windows, windows_arches);
    }

    // FreeBSD was added in v1.3.14
    let has_freebsd = match version {
        VersionSpec::Version(version) => *version >= Version::new(1, 3, 14),
        VersionSpec::Alias(alias) => alias == "latest",
        _ => true,
    };

    if has_freebsd {
        targets.insert(HostOS::FreeBSD, vec![HostArch::X64, HostArch::Arm64]);
    }

    check_supported_os_and_arch(NAME, env, targets)?;

    let arch = match env.arch {
        HostArch::Arm64 => "aarch64",
        HostArch::X64 => "x64",
        _ => unreachable!(),
    };

    // x64 builds before v1.4 require AVX2 (x86-64 level 3), otherwise the baseline build
    // is needed. Foreign platforms have no level, so use the default build.
    let baseline = env.arch == HostArch::X64
        && env
            .details
            .as_ref()
            .and_then(|details| details.x64_level)
            .is_some_and(|level| level < 3);
    let baseline_suffix = if baseline { "-baseline" } else { "" };

    let mut libc_suffix = "";

    if env.os.is_linux() && env.libc == HostLibc::Musl {
        // musl builds were added in v1.1.35
        if let VersionSpec::Version(version) = version
            && *version < Version::new(1, 1, 35)
        {
            return Err(plugin_err!(PluginError::Message(
                "musl is only supported for Bun v1.1.35 and above.".into()
            )));
        }

        libc_suffix = "-musl";
    }

    let prefix = match env.os {
        HostOS::Linux => format!("bun-linux-{arch}{libc_suffix}{baseline_suffix}"),
        HostOS::MacOS => format!("bun-darwin-{arch}{baseline_suffix}"),
        HostOS::Windows => format!("bun-windows-{arch}{baseline_suffix}"),
        HostOS::FreeBSD => format!("bun-freebsd-{arch}{baseline_suffix}"),
        _ => unreachable!(),
    };

    let filename = format!("{prefix}.zip");
    let mut host = get_tool_config::<BunToolConfig>()?.dist_url;

    // canary - bun-v1.2.3
    if version.is_canary() {
        host = host.replace("bun-v{version}", "{version}");
    };

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(prefix),
        download_url: host
            .replace("{version}", &version.to_string())
            .replace("{file}", &filename),
        download_name: Some(filename),
        // Checksums are not consistently updated
        checksum_url: if version.is_canary() {
            None
        } else {
            Some(
                host.replace("{version}", &version.to_string())
                    .replace("{file}", "SHASUMS256.txt"),
            )
        },
        variant: baseline.then(|| "baseline".into()),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    let bunx = ExecutableConfig {
        // `bunx` isn't a real binary provided by Bun so we can't symlink it.
        // Instead, it's simply the `bun` binary named `bunx` and Bun toggles
        // functionality based on `args[0]`.
        exe_link_path: Some(env.os.get_exe_name("bun").into()),

        // The approach doesn't work for shims since we use child processes,
        // so execute `bun x` instead (notice the space).
        shim_before_args: Some(StringOrVec::String("x".into())),

        ..Default::default()
    };

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                "bun".into(),
                ExecutableConfig::new_primary(env.os.get_exe_name("bun")),
            ),
            ("bunx".into(), bunx),
        ]),
        // Where `bun add --global` links bins, in Bun's lookup order
        globals_lookup_dirs: vec![
            "$BUN_INSTALL_BIN".into(),
            append_env_bin_dir("BUN_INSTALL")?,
            "$XDG_CACHE_HOME/.bun/bin".into(),
            "$HOME/.bun/bin".into(),
        ],
        ..Default::default()
    }))
}
