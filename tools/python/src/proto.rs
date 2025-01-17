use crate::version::from_python_version;
use extism_pdk::*;
use proto_pdk::*;
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn host_log(input: Json<HostLogInput>);
}

static NAME: &str = "Python";

#[plugin_fn]
pub fn register_tool(Json(_): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 42, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Message("Pre-builds are provided by astral-sh/python-build-standalone, which may not support all versions.".into()),
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".python-version".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/python/cpython")?;
    let regex = Regex::new(
        r"v?(?<major>[0-9]+)\.(?<minor>[0-9]+)(?:\.(?<patch>[0-9]+))?(?:(?<pre>a|b|c|rc)(?<preid>[0-9]+))?",
    )
    .unwrap();

    let tags = tags
        .into_iter()
        .filter_map(|tag| {
            if tag == "legacy-trunk" {
                None
            } else {
                from_python_version(tag, &regex)
            }
        })
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

// #[plugin_fn]
// pub fn native_install(
//     Json(input): Json<NativeInstallInput>,
// ) -> FnResult<Json<NativeInstallOutput>> {
//     let mut output = NativeInstallOutput::default();
//     let env = get_host_environment()?;

//     // https://github.com/pyenv/pyenv/tree/master/plugins/python-build
//     if command_exists(&env, "python-build") {
//         host_log!("Building with `python-build` instead of downloading a pre-built");

//         let result = exec_command!(
//             inherit,
//             "python-build",
//             [
//                 input.context.version.as_str(),
//                 input.install_dir.real_path().to_str().unwrap(),
//             ]
//         );

//         output.installed = result.exit_code == 0;
//     } else {
//         output.skip_install = true;
//     }

//     Ok(Json(output))
// }

#[derive(Deserialize)]
struct ReleaseEntry {
    download: String,
    checksum: Option<String>,
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let version = &input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: NAME.into()
        }));
    }

    let releases: BTreeMap<Version, BTreeMap<String, ReleaseEntry>> = fetch_json(
        "https://raw.githubusercontent.com/moonrepo/plugins/master/tools/python/releases.json",
    )?;

    let Some(release_triples) = version.as_version().and_then(|v| releases.get(v)) else {
        return Err(plugin_err!(
            "No pre-built available for version <hash>{}</hash> (via <url>https://github.com/astral-sh/python-build-standalone</url>)! Try installing another version for the time being.",
            version
        ));
    };

    let triple = get_target_triple(&env, NAME)?;

    let Some(release) = release_triples.get(&triple) else {
        return Err(plugin_err!(
            "No pre-built available for architecture <id>{}</id>!",
            triple
        ));
    };

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("python".into()),
        checksum_url: release.checksum.clone(),
        download_url: release.download.clone(),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[derive(Deserialize)]
struct PythonManifest {
    // python_exe: String,
    // python_major_minor_version: String,
    python_paths: HashMap<String, String>,
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let mut exe_path = env
        .os
        .for_native("install/bin/python", "install/python.exe")
        .to_owned();
    let mut exes_dir = env
        .os
        .for_native("install/bin", "install/Scripts")
        .to_owned();

    // Manifest is only available for pre-builts
    let manifest_path = input.context.tool_dir.join("PYTHON.json");

    if manifest_path.exists() {
        let manifest: PythonManifest = json::from_slice(&fs::read(manifest_path)?)?;

        if let Some(dir) = manifest.python_paths.get("scripts") {
            dir.clone_into(&mut exes_dir);
        }
    }

    // When on Unix, the executable returned from `PYTHON.json` is `pythonX.X`,
    // but this causes issues with our bin linking strategy, as the version in the
    // file name can be different than the one resolved, resulting in invalid
    // symlinks. To work around this, we can use `pythonX` instead, if `python`
    // itself doesn't exist (which is true for some versions).
    if !env.os.is_windows() && !input.context.tool_dir.join(&exe_path).exists() {
        if let Some(version) = input.context.version.as_version() {
            exe_path = format!("install/bin/python{}", version.major);
        }
    }

    Ok(Json(LocateExecutablesOutput {
        globals_lookup_dirs: vec![format!("$TOOL_DIR/{exes_dir}"), "$HOME/.local/bin".into()],
        exes: HashMap::from_iter([
            ("python".into(), ExecutableConfig::new_primary(exe_path)),
            (
                "pip".into(),
                ExecutableConfig {
                    no_bin: true,
                    shim_before_args: Some(StringOrVec::Vec(vec!["-m".into(), "pip".into()])),
                    ..ExecutableConfig::default()
                },
            ),
        ]),
        exes_dir: Some(exes_dir.into()),
        ..LocateExecutablesOutput::default()
    }))
}
