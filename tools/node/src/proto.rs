use crate::config::NodePluginConfig;
use extism_pdk::*;
use node_common::{NodeDistLTS, NodeDistVersion, VoltaField};
use nodejs_package_json::PackageJson;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

static NAME: &str = "Node.js";

#[plugin_fn]
pub fn register_tool(Json(_): Json<ToolMetadataInput>) -> FnResult<Json<ToolMetadataOutput>> {
    Ok(Json(ToolMetadataOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        config_schema: Some(SchemaBuilder::build_root::<NodePluginConfig>()),
        minimum_proto_version: Some(Version::new(0, 42, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..ToolMetadataOutput::default()
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![
            ".nvmrc".into(),
            ".node-version".into(),
            "package.json".into(),
        ],
        ignore: vec!["node_modules".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    if input.file == "package.json" {
        if let Ok(mut package_json) = json::from_str::<PackageJson>(&input.content) {
            if let Some(engines) = package_json.engines {
                if let Some(constraint) = engines.get("node") {
                    version = Some(UnresolvedVersionSpec::parse(constraint)?);
                }
            }

            if version.is_none() {
                if let Some(volta_raw) = package_json.other_fields.remove("volta") {
                    let volta: VoltaField = json::from_value(volta_raw)?;

                    if let Some(volta_node_version) = volta.node {
                        version = Some(UnresolvedVersionSpec::parse(volta_node_version)?);
                    }
                }
            }
        }
    } else {
        for line in input.content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            } else {
                version = Some(UnresolvedVersionSpec::parse(line)?);
                break;
            }
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let mut output = LoadVersionsOutput::default();
    let response: Vec<NodeDistVersion> =
        fetch_json("https://nodejs.org/download/release/index.json")?;

    for (index, item) in response.iter().enumerate() {
        let version = UnresolvedVersionSpec::parse(&item.version[1..])?;

        // First item is always the latest
        if index == 0 {
            output.latest = Some(version.clone());
        }

        if let NodeDistLTS::Name(alias) = &item.lts {
            let alias = alias.to_lowercase();

            // The first encounter of an lts is the latest stable
            if !output.aliases.contains_key("stable") {
                output.aliases.insert("stable".into(), version.clone());
            }

            // The first encounter of an lts is the latest version for that alias
            if !output.aliases.contains_key(&alias) {
                output.aliases.insert(alias.clone(), version.clone());
            }
        }

        output.versions.push(version.to_resolved_spec());
    }

    output
        .aliases
        .insert("latest".into(), output.latest.clone().unwrap());

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    if let UnresolvedVersionSpec::Alias(alias) = input.initial {
        let candidate = if alias == "node" {
            "latest"
        } else if alias == "lts" || alias == "lts-latest" || alias == "lts-*" || alias == "lts/*" {
            "stable"
        } else if alias.starts_with("lts-") || alias.starts_with("lts/") {
            &alias[4..]
        } else {
            return Ok(Json(output));
        };

        output.candidate = UnresolvedVersionSpec::parse(candidate).ok();
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn build_instructions(
    Json(input): Json<BuildInstructionsInput>,
) -> FnResult<Json<BuildInstructionsOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        NAME,
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64, HostArch::Arm, HostArch::Powerpc64, HostArch::S390x],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64, HostArch::X86, HostArch::Arm64],
        ],
    )?;

    let version = input.context.version;

    let mut output = BuildInstructionsOutput {
        help_url: Some("https://github.com/nodejs/node/blob/main/BUILDING.md".into()),
        source: Some(SourceLocation::Archive(ArchiveSource {
            url: format!("https://github.com/nodejs/node/archive/refs/tags/v{version}.tar.gz"),
            prefix: Some(format!("node-{version}")),
        })),
        system_dependencies: vec![
            SystemDependency::for_pm(
                HostPackageManager::Apt,
                ["python3", "python3-pip", "g++-12", "gcc-12", "make"],
            ),
            SystemDependency::for_pm(
                HostPackageManager::Dnf,
                ["python3", "python3-pip", "gcc-c++", "make"],
            ),
            SystemDependency::for_pm(
                HostPackageManager::Yum,
                ["python3", "python3-pip", "gcc-c++", "make"],
            ),
            SystemDependency::for_pm(
                HostPackageManager::Pacman,
                ["python3", "python3-pip", "gcc", "make"],
            ),
        ],
        requirements: vec![
            BuildRequirement::XcodeCommandLineTools,
            BuildRequirement::CommandExistsOnPath("python".into()),
            BuildRequirement::CommandExistsOnPath("make".into()),
        ],
        ..Default::default()
    };

    if env.os.is_windows() {
        output.requirements.push(BuildRequirement::ManualIntercept(
            "https://github.com/nodejs/node/blob/main/BUILDING.md#windows-prerequisites".into(),
        ));
        output
            .instructions
            .push(BuildInstruction::RunCommand(Box::new(
                CommandInstruction::new(".\\vcbuild", ["full-icu"]),
            )));
    } else {
        output.instructions.extend(vec![
            BuildInstruction::SetEnvVar("CXX".into(), "g++-12".into()),
            BuildInstruction::RunCommand(Box::new(CommandInstruction::new(
                "./configure",
                ["--with-intl", "full-icu"],
            ))),
            BuildInstruction::RunCommand(Box::new(CommandInstruction::new("make", ["-j4"]))),
        ]);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        NAME,
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64, HostArch::Arm, HostArch::Powerpc64, HostArch::S390x],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64, HostArch::X86, HostArch::Arm64],
        ],
    )?;

    let mut version = input.context.version;
    let mut host = get_tool_config::<NodePluginConfig>()?.dist_url;

    let mut arch: String = match env.arch {
        HostArch::Arm => "armv7l".into(),
        HostArch::Arm64 => "arm64".into(),
        HostArch::Powerpc64 => {
            if env.os.is_linux() {
                "ppc64le".into()
            } else {
                "ppc64".into()
            }
        }
        HostArch::S390x => "s390x".into(),
        HostArch::X64 => "x64".into(),
        HostArch::X86 => "x86".into(),
        _ => unreachable!(),
    };

    // Arm64 support was added after v16, but M1/M2 machines can
    // run x64 binaries via Rosetta. This is a compat hack!
    if env.arch == HostArch::Arm64 {
        let m1_compat_version = Version::new(16, 0, 0);

        if version.as_version().unwrap_or(&m1_compat_version).major < 16 {
            arch = "x64".into();
        }
    }

    // When canary, extract the latest version from the index
    if version.is_canary() {
        let response: Vec<NodeDistVersion> =
            fetch_json("https://nodejs.org/download/nightly/index.json")?;
        let file_to_match = match env.os {
            HostOS::Linux => format!("linux-{arch}"),
            HostOS::MacOS => format!("osx-{arch}-tar"),
            HostOS::Windows => format!("win-{arch}-zip"),
            _ => unreachable!(),
        };

        let entry = response
            .iter()
            .find(|row| row.files.iter().any(|file| file == &file_to_match))
            .unwrap_or(&response[0]);

        host = host.replace("/release/", "/nightly/");
        version = VersionSpec::parse(&entry.version)?;
    }

    let prefix = match env.os {
        HostOS::Linux => format!("node-v{version}-linux-{arch}"),
        HostOS::MacOS => format!("node-v{version}-darwin-{arch}"),
        HostOS::Windows => format!("node-v{version}-win-{arch}"),
        _ => unreachable!(),
    };

    let filename = if env.os.is_windows() {
        format!("{prefix}.zip")
    } else {
        format!("{prefix}.tar.xz")
    };

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(prefix),
        download_url: host
            .replace("{version}", &version.to_string())
            .replace("{file}", &filename),
        download_name: Some(filename),
        checksum_url: Some(
            host.replace("{version}", &version.to_string())
                .replace("{file}", "SHASUMS256.txt"),
        ),
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "node".into(),
            ExecutableConfig::new_primary(if env.os.is_windows() {
                "node.exe"
            } else {
                "bin/node"
            }),
        )]),
        exes_dir: Some(if env.os.is_windows() {
            ".".into()
        } else {
            "bin".into()
        }),
        globals_lookup_dirs: vec!["$PROTO_HOME/tools/node/globals/bin".into()],
        ..LocateExecutablesOutput::default()
    }))
}

#[plugin_fn]
pub fn post_install(Json(input): Json<InstallHook>) -> FnResult<()> {
    let config = get_tool_config::<NodePluginConfig>()?;

    if !config.bundled_npm
        || input
            .passthrough_args
            .iter()
            .any(|arg| arg == "--no-bundled-npm")
    {
        return Ok(());
    }

    debug!("Installing npm that comes bundled with Node.js");

    let mut args = vec!["install", "npm", "bundled"];

    let passthrough_args = input
        .passthrough_args
        .iter()
        .filter_map(|arg| {
            if arg.as_str() == "--no-bundled-npm" {
                None
            } else {
                Some(arg.as_str())
            }
        })
        .collect::<Vec<_>>();

    if !passthrough_args.is_empty() {
        args.push("--");
        args.extend(passthrough_args);
    }

    exec_command!(inherit, "proto", args);

    Ok(())
}
