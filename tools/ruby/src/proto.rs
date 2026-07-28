use extism_pdk::*;
use proto_pdk::*;
use serde::Deserialize;
use std::collections::HashMap;
use tool_common::enable_tracing;

#[derive(Deserialize)]
struct GitHubAsset {
    browser_download_url: String,
    name: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubAsset>,
    tag_name: String,
}

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
        minimum_proto_version: Some(Version::new(0, 59, 0)),
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

    if let Some(source) = find_prebuilt_source(&env, &version)? {
        return Ok(Json(BuildInstructionsOutput {
            source: Some(source),
            ..BuildInstructionsOutput::default()
        }));
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

fn find_prebuilt_source(
    env: &HostEnvironment,
    version: &VersionSpec,
) -> AnyResult<Option<SourceLocation>> {
    let Some(platform) = get_prebuilt_platform(env) else {
        return Ok(None);
    };

    let version = version.to_string();
    let filename = format!("ruby-{version}.{platform}.tar.gz");
    const PAGE_SIZE: usize = 100;
    let mut page = 1;

    loop {
        let releases: Vec<GitHubRelease> = fetch_json(format!(
            "https://api.github.com/repos/jdx/ruby/releases?per_page={PAGE_SIZE}&page={page}"
        ))?;
        let is_last_page = releases.len() < PAGE_SIZE;

        if let Some(source) = select_prebuilt_source(releases, &filename, &version) {
            return Ok(Some(source));
        }

        if is_last_page {
            return Ok(None);
        }

        page += 1;
    }
}

fn select_prebuilt_source(
    releases: Vec<GitHubRelease>,
    filename: &str,
    version: &str,
) -> Option<SourceLocation> {
    releases
        .into_iter()
        .find(|release| release.tag_name == version)?
        .assets
        .into_iter()
        .find(|asset| asset.name == filename)
        .map(|asset| {
            SourceLocation::Archive(ArchiveSource {
                url: asset.browser_download_url,
                prefix: Some(format!("ruby-{version}")),
            })
        })
}

fn get_prebuilt_platform(env: &HostEnvironment) -> Option<&'static str> {
    match (env.os, env.arch) {
        (HostOS::Linux, HostArch::X64) => Some("x86_64_linux"),
        (HostOS::Linux, HostArch::Arm64) => Some("arm64_linux"),
        (HostOS::MacOS, HostArch::Arm64) => Some("macos"),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_jdx_supported_platforms() {
        for (os, arch, expected) in [
            (HostOS::Linux, HostArch::X64, Some("x86_64_linux")),
            (HostOS::Linux, HostArch::Arm64, Some("arm64_linux")),
            (HostOS::MacOS, HostArch::Arm64, Some("macos")),
            (HostOS::MacOS, HostArch::X64, None),
            (HostOS::Windows, HostArch::X64, None),
        ] {
            assert_eq!(
                get_prebuilt_platform(&HostEnvironment {
                    os,
                    arch,
                    ..HostEnvironment::default()
                }),
                expected
            );
        }
    }

    #[test]
    fn selects_matching_release_asset() {
        let source = select_prebuilt_source(
            vec![GitHubRelease {
                assets: vec![GitHubAsset {
                    browser_download_url: "https://example.com/ruby.tar.gz".into(),
                    name: "ruby-3.4.9.arm64_linux.tar.gz".into(),
                }],
                tag_name: "3.4.9".into(),
            }],
            "ruby-3.4.9.arm64_linux.tar.gz",
            "3.4.9",
        );

        assert_eq!(
            source,
            Some(SourceLocation::Archive(ArchiveSource {
                url: "https://example.com/ruby.tar.gz".into(),
                prefix: Some("ruby-3.4.9".into()),
            }))
        );
    }

    #[test]
    fn skips_release_without_matching_asset() {
        let source = select_prebuilt_source(
            vec![GitHubRelease {
                assets: vec![],
                tag_name: "3.4.9".into(),
            }],
            "ruby-3.4.9.macos.tar.gz",
            "3.4.9",
        );

        assert_eq!(source, None);
    }

    #[test]
    fn skips_missing_release() {
        let source = select_prebuilt_source(vec![], "ruby-3.1.0.macos.tar.gz", "3.1.0");

        assert_eq!(source, None);
    }
}
