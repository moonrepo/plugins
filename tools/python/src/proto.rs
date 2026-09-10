use crate::config::PythonToolConfig;
use crate::version::{from_python_tag, from_python_version, to_python_version};
use extism_pdk::*;
use proto_pdk::*;
use regex::Regex;
use schematic::SchemaBuilder;
use std::collections::{HashMap, HashSet};
use tool_common::{enable_tracing, registry::*};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn host_log(input: Json<HostLogInput>);
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

static NAME: &str = "Python";

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 60, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<PythonToolConfig>(),
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
    let env = get_host_environment()?;
    let regex = Regex::new(
        r"v?(?<major>[0-9]+)\.(?<minor>[0-9]+)(?:\.(?<patch>[0-9]+))?(?:(?<pre>a|b|c|rc)(?<preid>[0-9]+))?",
    )
    .unwrap();

    let tags = load_git_tags("https://github.com/python/cpython")?
        .into_iter()
        .filter_map(|tag| {
            if tag == "legacy-trunk" {
                None
            } else {
                from_python_tag(tag, &regex)
            }
        })
        .collect::<Vec<_>>();

    let mut output = LoadVersionsOutput::from(tags)?;
    let mut versions = HashSet::<VersionSpec>::from_iter(output.versions);

    // Include our build specific versions, as these are not official
    versions.extend(fetch_versions(env, "python", true)?);

    output.versions = versions.into_iter().collect();

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let config = get_tool_config::<PythonToolConfig>()?;
    let mut output = ResolveVersionOutput::default();

    let UnresolvedVersionSpec::Version(initial) = &input.initial else {
        return Ok(Json(output));
    };

    // Normalize prereleases to the registry format
    let version = Version::parse(from_python_version(&initial.to_string()))?;

    if &version != initial {
        output.candidate = Some(UnresolvedVersionSpec::Version(version.clone()));
    }

    // If we have a full semantic version without a build,
    // fetch the available release and see if we have a build to use
    if config.use_latest_build
        && version.build.is_none()
        && let Ok(release) = fetch_release("python", &version)
        && let Some(build_id) = release.builds.keys().next()
    {
        output.version = Some(VersionSpec::parse(format!("{version}+{build_id}"))?);
    }

    Ok(Json(output))
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

    let python_version = match version.as_version() {
        Some(version) => to_python_version(version),
        None => version.to_string(),
    };

    let output = BuildInstructionsOutput {
        help_url: Some(
            "https://github.com/pyenv/pyenv/blob/master/plugins/python-build/README.md".into(),
        ),
        system_dependencies: vec![
            SystemDependency::for_pm(
                HostPackageManager::Apk,
                "build-base libffi-dev openssl-dev bzip2-dev zlib-dev xz-dev readline-dev sqlite-dev tk-dev zstd-dev"
                    .split(' ')
                    .collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Apt,
                "make build-essential libssl-dev zlib1g-dev libbz2-dev libreadline-dev libsqlite3-dev curl libncursesw5-dev xz-utils tk-dev libxml2-dev libxmlsec1-dev libffi-dev liblzma-dev"
                    .split(' ')
                    .collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Brew,
                "openssl readline sqlite3 xz tcl-tk@8 libb2 zstd zlib pkgconfig"
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
                "base-devel openssl zlib xz tk zstd"
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
                id: Id::new("python-build")?,
                exe: "plugins/python-build/bin/python-build".into(),
                git: GitSource {
                    url: "https://github.com/pyenv/pyenv.git".into(),
                    ..Default::default()
                },
                ..Default::default()
            })),
            BuildInstruction::RunCommand(Box::new(CommandInstruction::with_builder(
                "python-build",
                ["--verbose", python_version.as_str(), "."],
            ))),
        ],
        ..Default::default()
    };

    Ok(Json(output))
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

    let make_error = || {
        plugin_err!(
            "No pre-built available for <hash>{spec}</hash> on <id>{}-{}</id> (via <url>https://github.com/astral-sh/python-build-standalone</url>)! Try building from source with <shell>--build</shell>.",
            env.os,
            env.arch,
        )
    };

    let Some(version) = spec.as_version() else {
        return Err(make_error());
    };

    let release = fetch_release("python", version)?;

    let Some(mut output) = release.create_download_prebuilt(env, version) else {
        return Err(make_error());
    };

    // Older releases are not "install only" and nest the files in a sub-folder
    output.archive_prefix = Some(
        if output
            .download_name
            .as_ref()
            .is_some_and(|name| name.contains("install_only"))
        {
            "python".into()
        } else {
            "python/install".into()
        },
    );

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let mut exe_path = env.os.for_native("bin/python", "python.exe").to_owned();
    let mut exes_dir = env.os.for_native("bin", "Scripts").to_owned();

    // Backwards compatibility for the old pre-built implementation
    if input.install_dir.join("PYTHON.json").exists() || input.install_dir.join("install").exists()
    {
        exe_path = format!("install/{exe_path}");
        exes_dir = format!("install/{exes_dir}");
    }

    // When on Unix, the executable returned from `PYTHON.json` is `pythonX.X`,
    // but this causes issues with our bin linking strategy, as the version in the
    // file name can be different than the one resolved, resulting in invalid
    // symlinks. To work around this, we can use `pythonX` instead, if `python`
    // itself doesn't exist (which is true for some versions).
    if !env.os.is_windows()
        && !input.install_dir.join(&exe_path).exists()
        && let Some(version) = input.context.version.as_version()
    {
        exe_path = format!("{exe_path}{}", version.major);
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
                    ..Default::default()
                },
            ),
        ]),
        exes_dirs: vec![exes_dir.into()],
        ..Default::default()
    }))
}
