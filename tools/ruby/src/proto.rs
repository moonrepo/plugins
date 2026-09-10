use crate::config::RubyToolConfig;
use crate::version::*;
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::{HashMap, HashSet};
use tool_common::{enable_tracing, registry::*};

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
    fn send_request(input: Json<SendRequestInput>) -> Json<SendRequestOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: "Ruby".into(),
        type_of: PluginType::Language,
        default_install_strategy: InstallStrategy::DownloadPrebuilt,
        minimum_proto_version: Some(Version::new(0, 60, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Message("Windows is currently not supported.".into()),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<RubyToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".ruby-version".into()],
        ignore: vec!["vendor".into()],
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let env = get_host_environment()?;

    let tags = load_git_tags("https://github.com/ruby/ruby")?
        .into_iter()
        .filter_map(|tag| {
            if let Some(tag) = tag.strip_prefix('v') {
                // First 2 underscores are the separators between the major,
                // minor, and patch digits, while the remaining underscores
                // are used in the pre/build metadata
                let version = tag.replacen('_', ".", 2).replace('_', "-");

                // Very old versions that we don't need to support
                if version.starts_with('0') || version.starts_with('1') {
                    None
                } else {
                    Some(from_ruby_version(&version))
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut output = LoadVersionsOutput::from(tags)?;
    let mut versions = HashSet::<VersionSpec>::from_iter(output.versions);

    // Include our build specific versions, as these are not official
    versions.extend(fetch_versions(env, "ruby", true)?);

    output.versions = versions.into_iter().collect();

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let config = get_tool_config::<RubyToolConfig>()?;
    let mut output = ResolveVersionOutput::default();

    let UnresolvedVersionSpec::Version(initial) = &input.initial else {
        return Ok(Json(output));
    };

    // Normalize prereleases to the registry format
    let version = Version::parse(from_ruby_version(&initial.to_string()))?;

    if &version != initial {
        output.candidate = Some(UnresolvedVersionSpec::Version(version.clone()));
    }

    // If we have a full semantic version without a build,
    // fetch the available release and see if we have a build to use
    if config.use_latest_build
        && version.build.is_none()
        && let Ok(release) = fetch_release("ruby", &version)
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

    let ruby_version = match version.as_version() {
        Some(version) => to_ruby_version(version),
        None => version.to_string(),
    };

    let output = BuildInstructionsOutput {
        help_url: Some(
            "https://github.com/rbenv/ruby-build/wiki".into(),
        ),
        system_dependencies: vec![
            SystemDependency::for_pm(
                HostPackageManager::Apk,
                "build-base gcc patch bzip2 libffi-dev openssl-dev ncurses-dev gdbm-dev zlib-dev readline-dev yaml-dev".split(' ').collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Apt,
                "build-essential autoconf libssl-dev libyaml-dev zlib1g-dev libffi-dev libgmp-dev rustc patch libreadline6-dev libncurses5-dev libgdbm6 libgdbm-dev libdb-dev".split(' ').collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Brew,
                "openssl@3 readline libyaml gmp autoconf".split(' ').collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Dnf,
                "autoconf gcc rust patch make bzip2 openssl-devel libyaml-devel libffi-devel readline-devel gdbm-devel ncurses-devel zlib-devel perl-FindBin".split(' ').collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Pacman,
                "base-devel rust libffi libyaml openssl zlib".split(' ').collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Pkg,
                "devel/autoconf devel/bison devel/patch lang/gcc lang/rust databases/gdbm devel/gmake devel/libffi textproc/libyaml devel/ncurses security/openssl devel/readline".split(' ').collect::<Vec<_>>(),
            ),
            SystemDependency::for_pm(
                HostPackageManager::Yum,
                "autoconf gcc patch bzip2 openssl-devel libffi-devel readline-devel zlib-devel gdbm-devel ncurses-devel tar".split(' ').collect::<Vec<_>>(),
            ),
        ],
        requirements: vec![BuildRequirement::XcodeCommandLineTools],
        instructions: vec![
            BuildInstruction::InstallBuilder(Box::new(BuilderInstruction {
                id: Id::new("ruby-build")?,
                exe: "bin/ruby-build".into(),
                git: GitSource {
                    url: "https://github.com/rbenv/ruby-build.git".into(),
                    ..Default::default()
                },
                ..Default::default()
            })),
            BuildInstruction::RunCommand(Box::new(CommandInstruction::with_builder(
                "ruby-build",
                ["--verbose", ruby_version.as_str(), "."],
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

    let make_error = || {
        plugin_err!(
            "No pre-built available for <hash>{spec}</hash> on <id>{}-{}</id>! Try building from source with <shell>--build</shell>.",
            env.os,
            env.arch,
        )
    };

    let Some(version) = spec.as_version() else {
        return Err(make_error());
    };

    let release = fetch_release("ruby", version)?;

    let Some(mut output) = release.create_download_prebuilt(env, version) else {
        return Err(make_error());
    };

    // Does not include the build suffix!
    output.archive_prefix = Some(format!("ruby-{}", to_ruby_version(version)));

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                "ruby".into(),
                ExecutableConfig::new_primary(env.os.get_exe_name("bin/ruby")),
            ),
            (
                "rake".into(),
                ExecutableConfig::new(env.os.get_exe_name("bin/rake")),
            ),
            (
                "gem".into(),
                ExecutableConfig::new(env.os.get_exe_name("bin/gem")),
            ),
            (
                "bundle".into(),
                ExecutableConfig::new(env.os.get_exe_name("bin/bundle")),
            ),
            (
                "irb".into(),
                ExecutableConfig::new(env.os.get_exe_name("bin/irb")),
            ),
        ]),
        exes_dirs: vec!["bin".into()],
        globals_lookup_dirs: vec![],
        ..Default::default()
    }))
}
