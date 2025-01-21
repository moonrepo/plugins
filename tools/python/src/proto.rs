use crate::version::from_python_version;
use extism_pdk::*;
use proto_pdk::*;
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

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

#[plugin_fn]
pub fn build_instructions(
    Json(input): Json<BuildInstructionsInput>,
) -> FnResult<Json<BuildInstructionsOutput>> {
    let env = get_host_environment()?;
    let version = input.context.version;

    if env.os.is_windows() {
        return Err(PluginError::UnsupportedWindowsBuild.into());
    }

    // check_supported_os_and_arch(
    //     NAME,
    //     &env,
    //     permutations! [
    //         HostOS::Linux => [HostArch::X86, HostArch::X64, HostArch::Arm, HostArch::Arm64, HostArch::S390x, HostArch::Riscv64, HostArch::Powerpc64],
    //         HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
    //         // HostOS::Windows => [HostArch::X86, HostArch::X64],
    //     ],
    // )?;

    let output = BuildInstructionsOutput {
        help_url: Some(
            "https://github.com/pyenv/pyenv/blob/master/plugins/python-build/README.md".into(),
        ),
        system_dependencies: vec![
            SystemDependency::for_pm(
                HostPackageManager::Apt,
                "build-essential libssl-dev zlib1g-dev libbz2-dev libreadline-dev libsqlite3-dev curl git libncursesw5-dev xz-utils tk-dev libxml2-dev libxmlsec1-dev libffi-dev liblzma-dev"
                    .split(' ')
                    .collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Brew,
                "openssl readline sqlite3 xz zlib tcl-tk@8"
                    .split(' ')
                    .collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Dnf,
                "make gcc patch zlib-devel bzip2 bzip2-devel readline-devel sqlite sqlite-devel openssl-devel tk-devel libffi-devel xz-devel libuuid-devel gdbm-libs libnsl2"
                    .split(' ')
                    .collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Pacman,
                "base-devel openssl zlib xz tk"
                    .split(' ')
                    .collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Yum,
                "gcc make patch zlib-devel bzip2 bzip2-devel readline-devel sqlite sqlite-devel openssl-devel tk-devel libffi-devel xz-devel"
                    .split(' ')
                    .collect::<Vec<_>>(),
            ),
        ],
        requirements: vec![BuildRequirement::XcodeCommandLineTools],
        instructions: vec![
            BuildInstruction::InstallBuilder(Box::new(BuilderInstruction {
                id: "python-build".into(),
                exe: "plugins/python-build/bin/python-build".into(),
                git: GitSource {
                    url: "https://github.com/pyenv/pyenv.git".into(),
                    ..Default::default()
                },
            })),
            BuildInstruction::RunCommand(Box::new(CommandInstruction::with_builder(
                "python-build",
                ["--verbose", version.to_string().as_str(), "."],
            ))),
        ],
        ..Default::default()
    };

    Ok(Json(output))
}

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
            "No pre-built available for version <hash>{version}</hash> (via <url>https://github.com/astral-sh/python-build-standalone</url>)! Try building from source with <shell>--build</shell>.",
        ));
    };

    let triple = get_target_triple(&env, NAME)?;

    let Some(release) = release_triples.get(&triple) else {
        return Err(plugin_err!(
            "No pre-built available for architecture <id>{triple}</id>! Try building from source with <shell>--build</shell>."
        ));
    };

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("python/install".into()),
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
    let mut exe_path = env.os.for_native("bin/python", "python.exe").to_owned();
    let mut exes_dir = env.os.for_native("bin", "Scripts").to_owned();

    // Backwards compatibility for the old pre-built implementation
    if input.context.tool_dir.join("PYTHON.json").exists() {
        exe_path = env
            .os
            .for_native("install/bin/python", "install/python.exe")
            .to_owned();
        exes_dir = env
            .os
            .for_native("install/bin", "install/Scripts")
            .to_owned();
    }

    // When on Unix, the executable returned from `PYTHON.json` is `pythonX.X`,
    // but this causes issues with our bin linking strategy, as the version in the
    // file name can be different than the one resolved, resulting in invalid
    // symlinks. To work around this, we can use `pythonX` instead, if `python`
    // itself doesn't exist (which is true for some versions).
    if !env.os.is_windows() && !input.context.tool_dir.join(&exe_path).exists() {
        if let Some(version) = input.context.version.as_version() {
            exe_path = format!("{exe_path}{}", version.major);
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
