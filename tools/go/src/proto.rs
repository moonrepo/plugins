use crate::config::GoToolConfig;
use crate::version::{from_go_version, to_go_version};
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;
use tool_common::{append_env_bin_dir, enable_tracing};

static NAME: &str = "Go";

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
        schema: SchemaBuilder::build_root::<GoToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec!["go.mod".into(), "go.work".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let tags = load_git_tags("https://github.com/golang/go")?
        .iter()
        .filter_map(|tag| tag.strip_prefix("go"))
        .map(from_go_version)
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(tags)?))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut go = None;
    let mut toolchain = None;

    if input.file == "go.mod" || input.file == "go.work" {
        for line in input.content.lines() {
            if let Some(value) = line.strip_prefix("go ") {
                go = Some(value.trim());
            } else if let Some(value) = line.strip_prefix("toolchain ") {
                // Names like `default` aren't versions
                toolchain = value
                    .trim()
                    .strip_prefix("go")
                    .filter(|value| value.starts_with(|c: char| c.is_ascii_digit()));
            }
        }
    }

    // Go runs at least the `toolchain` version, and downloads
    // it itself when the running Go is older
    let version = match toolchain.or(go) {
        Some(value) => Some(UnresolvedVersionSpec::parse(format!(
            "^{}",
            from_go_version(value)
        ))?),
        None => None,
    };

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn build_instructions(
    Json(input): Json<BuildInstructionsInput>,
) -> FnResult<Json<BuildInstructionsOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        NAME,
        env,
        permutations! [
            HostOS::Linux => [
                HostArch::X64, HostArch::Arm64, HostArch::X86, HostArch::Arm, HostArch::S390x,
                HostArch::Riscv64, HostArch::Powerpc64, HostArch::LongArm64
            ],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64, HostArch::Arm64, HostArch::X86],
            HostOS::FreeBSD => [HostArch::X64, HostArch::Arm64, HostArch::X86],
        ],
    )?;

    let version = input.context.version;
    let tag = format!("go{}", to_go_version(&version));

    let output = BuildInstructionsOutput {
        help_url: Some("https://go.dev/doc/install/source".into()),
        // source: Some(SourceLocation::Git(GitSource {
        //     url: "https://go.googlesource.com/go".into(),
        //     reference: Some(tag),
        //     submodules: false,
        // })),
        source: Some(SourceLocation::Archive(ArchiveSource {
            url: format!("https://github.com/golang/go/archive/refs/tags/{tag}.tar.gz"),
            prefix: Some(format!("go-{tag}")),
        })),
        // Bootstraps with an existing Go
        requirements: vec![BuildRequirement::CommandExistsOnPath("go".into())],
        // `all.bash` would also run the entire test suite. The whole tree is
        // kept, as Go needs `src`, `pkg`, `lib`, and `go.env` in its GOROOT
        instructions: vec![BuildInstruction::RunCommand(Box::new({
            CommandInstruction {
                exe: if env.os.is_windows() {
                    "./make.bat"
                } else {
                    "./make.bash"
                }
                .into(),
                cwd: Some(input.install_dir.join("src")),
                ..Default::default()
            }
        }))],
        ..Default::default()
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        NAME,
        env,
        permutations! [
            HostOS::Linux => [
                HostArch::X64, HostArch::Arm64, HostArch::X86, HostArch::Arm, HostArch::S390x,
                HostArch::Riscv64, HostArch::Powerpc64, HostArch::LongArm64
            ],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64, HostArch::Arm64, HostArch::X86],
            HostOS::FreeBSD => [HostArch::X64, HostArch::Arm64, HostArch::X86],
        ],
    )?;

    let version = &input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: NAME.into()
        }));
    }

    let arch = match env.arch {
        HostArch::Arm => "armv6l",
        HostArch::Arm64 => "arm64",
        HostArch::X64 => "amd64",
        HostArch::X86 => "386", // i386
        HostArch::S390x => "s390x",
        HostArch::Riscv64 => "riscv64",
        HostArch::Powerpc64 => "ppc64le", // Little endian, like most Linux hosts
        HostArch::LongArm64 => "loong64",
        _ => unreachable!(),
    };

    let version = to_go_version(version);
    let prefix = match env.os {
        HostOS::Linux => format!("go{version}.linux-{arch}"),
        HostOS::FreeBSD => format!("go{version}.freebsd-{arch}"),
        HostOS::MacOS => format!("go{version}.darwin-{arch}"),
        HostOS::Windows => format!("go{version}.windows-{arch}"),
        _ => unreachable!(),
    };

    let filename = if env.os.is_windows() {
        format!("{prefix}.zip")
    } else {
        format!("{prefix}.tar.gz")
    };

    let host = get_tool_config::<GoToolConfig>()?.dist_url;

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some("go".into()),
        checksum_url: Some(
            host.replace("{version}", &version)
                .replace("{file}", &format!("{filename}.sha256")),
        ),
        download_url: host
            .replace("{version}", &version)
            .replace("{file}", &filename),
        download_name: Some(filename),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([
            (
                "go".into(),
                ExecutableConfig::new_primary(env.os.get_exe_name("bin/go")),
            ),
            (
                "gofmt".into(),
                ExecutableConfig::new(env.os.get_exe_name("bin/gofmt")),
            ),
        ]),
        // Not `$GOROOT/bin`, which is a Go installation, not where
        // `go install` writes, and would shadow this tool's `go`
        globals_lookup_dirs: vec![
            "$GOBIN".into(),
            append_env_bin_dir("GOPATH")?,
            "$HOME/go/bin".into(),
        ],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn activate_environment(
    Json(input): Json<ActivateEnvironmentInput>,
) -> FnResult<Json<ActivateEnvironmentOutput>> {
    let mut output = ActivateEnvironmentOutput::default();

    // An inherited `GOROOT` for another Go (set by goenv, for example)
    // makes this Go compile with the other's standard library
    if let Some(root) = input.context.tool_dir.to_real_path()? {
        output.env.insert("GOROOT".into(), root.to_string());
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn sync_shell_profile(
    Json(input): Json<SyncShellProfileInput>,
) -> FnResult<Json<SyncShellProfileOutput>> {
    let config = get_tool_config::<GoToolConfig>()?;

    Ok(Json(SyncShellProfileOutput {
        check_var: "GOBIN".into(),
        export_vars: Some(HashMap::from_iter([(
            "GOBIN".into(),
            "$HOME/go/bin".into(),
        )])),
        extend_path: Some(vec!["$GOBIN".into()]),
        skip_sync: !config.gobin
            || input
                .passthrough_args
                .iter()
                .any(|arg| arg.as_str() == "--no-gobin"),
    }))
}
