use extism_pdk::*;
use proto_pdk::*;
use std::collections::HashMap;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: "Ruby".into(),
        type_of: PluginType::Language,
        default_install_strategy: InstallStrategy::BuildFromSource,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Message("Windows is currently not supported.".into()),
        ..RegisterToolOutput::default()
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
        ..LocateExecutablesOutput::default()
    }))
}
