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

#[derive(Deserialize)]
struct PythonManifest {
    python_exe: String,
    python_major_minor_version: String,
    python_paths: HashMap<String, String>,
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        plugin_version: Some(env!("CARGO_PKG_VERSION").into()),
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

    let releases: BTreeMap<Version, BTreeMap<String, ReleaseEntry>> = fetch_url(
        "https://raw.githubusercontent.com/moonrepo/tools/master/tools/python/releases.json",
    )?;

    let Some(release_triples) = version.as_version().and_then(|v| releases.get(v)) else {
        return Err(plugin_err!(
            "No pre-built available for version <hash>{}</hash> (via <url>https://github.com/indygreg/python-build-standalone</url>)! Try installing another version for the time being.",
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

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let id = get_plugin_id()?;
    let mut exe_path = env
        .os
        .for_native("install/bin/python", "install/python.exe")
        .to_owned();
    let mut exes_dir = env
        .os
        .for_native("install/bin", "install/Scripts")
        .to_owned();
    let mut major_version = "3".to_owned();

    // Manifest is only available for pre-builts
    let manifest_path = input.context.tool_dir.join("PYTHON.json");

    if manifest_path.exists() {
        let manifest: PythonManifest = json::from_slice(&fs::read(manifest_path)?)?;

        exe_path = manifest.python_exe;

        if let Some(dir) = manifest.python_paths.get("scripts") {
            dir.clone_into(&mut exes_dir);
        }

        if let Some(index) = manifest.python_major_minor_version.find('.') {
            major_version = manifest.python_major_minor_version[0..index].to_string();
        }
    }
    // Otherwise this was built from source
    else if let Some(version) = input.context.version.as_version() {
        major_version = version.major.to_string();
    }

    // Create a secondary executable that includes the major version as a suffix
    // https://gregoryszorc.com/docs/python-build-standalone/main/quirks.html#no-pip-exe-on-windows
    let secondary = HashMap::from_iter([
        // python3
        (
            format!("{id}{major_version}"),
            ExecutableConfig::new(&exe_path),
        ),
        // pip
        (
            "pip".into(),
            ExecutableConfig {
                no_bin: true,
                shim_before_args: Some(StringOrVec::Vec(vec!["-m".into(), "pip".into()])),
                ..ExecutableConfig::default()
            },
        ),
        // pip3
        (
            format!("pip{major_version}"),
            ExecutableConfig {
                no_bin: true,
                shim_before_args: Some(StringOrVec::Vec(vec!["-m".into(), "pip".into()])),
                ..ExecutableConfig::default()
            },
        ),
    ]);

    Ok(Json(LocateExecutablesOutput {
        globals_lookup_dirs: vec![format!("$TOOL_DIR/{exes_dir}")],
        exes_dir: Some(exes_dir.into()),
        primary: Some(ExecutableConfig::new(exe_path)),
        secondary,
        ..LocateExecutablesOutput::default()
    }))
}
