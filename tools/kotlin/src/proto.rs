use crate::config::KotlinToolConfig;
use crate::legacy::LEGACY_RELEASES;
use crate::version::{download_names, versions_from_tags};
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "Kotlin";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 60, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        lock_options: ToolLockOptions {
            ignore_os_arch: true,
            ..Default::default()
        },
        unstable: Switch::Toggle(true),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<KotlinToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".kotlin-version".into(), ".sdkmanrc".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    for line in input.content.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();

        if line.is_empty() {
            continue;
        }

        let value = match input.file.as_str() {
            ".kotlin-version" => line,
            ".sdkmanrc" => {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };

                if key.trim() != "kotlin" {
                    continue;
                }

                let value = value.trim();

                if value.is_empty() {
                    return Err(plugin_err!("The Kotlin version in .sdkmanrc is empty."));
                }

                value
            }
            _ => break,
        };

        version = Some(UnresolvedVersionSpec::parse(value)?);
        break;
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    Ok(Json(versions_from_tags(load_git_tags(
        "https://github.com/JetBrains/kotlin",
    )?)?))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    // A two-part preview otherwise behaves like a minor-version range and can
    // select a newer stable release. Keep plain minor versions as ranges.
    let candidate = if matches!(
        &input.initial,
        UnresolvedVersionSpec::Requirement(req)
            if req.op == Op::Tilde && req.patch.is_none() && req.prerelease.is_some()
    ) {
        let requested = input.initial.to_partial_string();

        LEGACY_RELEASES
            .iter()
            .find(|(version, tag, _)| {
                *version != requested && tag.strip_prefix('v') == Some(requested.as_str())
            })
            .map(|(version, _, _)| UnresolvedVersionSpec::parse(version))
            .transpose()?
    } else {
        None
    };

    Ok(Json(ResolveVersionOutput {
        candidate,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let spec = &input.context.version;

    if spec.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: NAME.into()
        }));
    }

    if !matches!(env.os, HostOS::Linux | HostOS::MacOS | HostOS::Windows) {
        return Err(plugin_err!(PluginError::UnsupportedOS {
            tool: NAME.into(),
            os: env.os.to_string(),
        }));
    }

    let Some(version) = spec.as_version().filter(|version| version.scope.is_none()) else {
        return Err(plugin_err!(
            "Kotlin downloads require a resolved, unscoped release version."
        ));
    };

    let config = get_tool_config::<KotlinToolConfig>()?;
    let (tag, filename) = download_names(version);
    let download_url = config
        .dist_url
        .replace("{version}", &version.to_string())
        .replace("{tag}", &tag)
        .replace("{file}", &filename);

    // SHA-256 files first appeared in 1.9.0-RC; 1.9.0-Beta and older
    // compiler releases only provide the ZIP archive.
    let checksum_url = if version >= &Version::parse("1.9.0-RC")? {
        Some(format!("{download_url}.sha256"))
    } else {
        None
    };

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("kotlinc".into()),
        download_name: Some(filename),
        download_url,
        checksum_url,
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let launcher = |name: &str| {
        if env.os.is_windows() {
            format!("bin/{name}.bat")
        } else {
            format!("bin/{name}")
        }
    };

    let compiler = if input.install_dir.join(launcher("kotlinc")).exists() {
        "kotlinc"
    } else {
        "kotlinc-jvm"
    };
    let runner = if input.install_dir.join(launcher("kotlinr")).exists() {
        "kotlinr"
    } else if input.install_dir.join(launcher("kotlin")).exists() {
        "kotlin"
    } else {
        compiler
    };
    let mut exes = HashMap::from_iter([
        (
            "kotlin".into(),
            ExecutableConfig::new_primary(launcher(runner)),
        ),
        ("kotlinc".into(), ExecutableConfig::new(launcher(compiler))),
    ]);

    for name in [
        "kotlinr",
        "kotlinc-jvm",
        "kotlinc-js",
        "kotlinc-wasm",
        "kapt",
    ] {
        let path = launcher(name);

        if input.install_dir.join(&path).exists() {
            exes.insert(name.into(), ExecutableConfig::new(path));
        }
    }

    for exe in exes.values_mut() {
        // The scripts locate sibling launchers and libraries relative to
        // themselves. Use proto shims instead of standalone bin symlinks.
        exe.no_bin = true;
        exe.update_perms = env.os.is_unix();
    }

    Ok(Json(LocateExecutablesOutput {
        exes,
        exes_dirs: vec!["bin".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn activate_environment(
    Json(input): Json<ActivateEnvironmentInput>,
) -> FnResult<Json<ActivateEnvironmentOutput>> {
    let mut output = ActivateEnvironmentOutput::default();

    if let Some(home) = input.context.tool_dir.to_real_path()? {
        output.env.insert("KOTLIN_HOME".into(), home.to_string());
    }

    Ok(Json(output))
}
