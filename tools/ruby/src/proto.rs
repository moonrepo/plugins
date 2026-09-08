use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;
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
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".ruby-version".into()],
        ignore: vec!["vendor".into()],
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
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
                    Some(version)
                }
            } else {
                None
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
                ["--verbose", version.to_string().as_str(), "."],
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

    // The API does not return the release identifier or the archive prefix,
    // so we need to create those values manually
    // https://github.com/jdx/ruby/releases
    let mut release_id = format!("{}.{}.{}", version.major, version.minor, version.patch);

    if let Some(pre) = &version.prerelease {
        release_id.push('-');

        if pre.contains("preview") {
            release_id.push_str(&pre.replace(".", ""));
        } else {
            release_id.push_str(pre);
        }
    }

    // Prefix contains prerelease but not build
    let archive_prefix = format!("ruby-{release_id}");

    if let Some(build) = &version.build {
        release_id.push('-');
        release_id.push_str(build);
    }

    let Some(mut output) = release.create_download_prebuilt(release_id, env, version) else {
        return Err(make_error());
    };

    output.archive_prefix = Some(archive_prefix);

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
