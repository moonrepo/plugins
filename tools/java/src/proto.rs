use crate::config::{ArchiveType, Distribution, JavaToolConfig, PackageType};
use crate::foojay::{FoojayPackage, fetch_package_info, fetch_packages};
use crate::version::{from_java_version, to_java_version};
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use tool_common::enable_tracing;

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: "Java".into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<JavaToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec![".java-version".into(), ".sdkmanrc".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    let mut version = None;

    fn normalize_sdkman_version(value: &str) -> &str {
        let value = value.trim();

        if let Some((version, vendor)) = value.rsplit_once('-')
            && vendor.chars().all(|c| c.is_ascii_alphabetic())
        {
            version
        } else {
            value
        }
    }

    if input.file == ".sdkmanrc" || input.file == ".java-version" {
        for line in input.content.lines() {
            let line = line.trim();

            if line.is_empty() || input.file == ".sdkmanrc" && !line.starts_with("java=") {
                continue;
            }

            version = Some(UnresolvedVersionSpec::parse(normalize_sdkman_version(
                line.strip_prefix("java=").unwrap_or(line),
            ))?);

            break;
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let env = get_host_environment()?;
    let config = get_tool_config::<JavaToolConfig>()?;

    let versions = fetch_packages(&env, &config, None)?
        .into_iter()
        .map(|package| from_java_version(&package.java_version))
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from(versions)?))
}

fn find_package(packages: &[FoojayPackage]) -> Option<&FoojayPackage> {
    for archive in [
        ArchiveType::TarGz,
        ArchiveType::TarXz,
        ArchiveType::Tar,
        ArchiveType::Zip,
    ] {
        if let Some(package) = packages
            .iter()
            .find(|package| package.archive_type == archive)
        {
            return Some(package);
        }
    }

    None
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let version = &input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: "Java".into()
        }));
    }

    let env = get_host_environment()?;
    let config = get_tool_config::<JavaToolConfig>()?;
    let full_version = version.to_string();
    let short_version = to_java_version(version);

    // Load all matching packages
    let mut packages = fetch_packages(
        &env,
        &config,
        if version.is_latest() {
            None
        } else {
            Some(&short_version)
        },
    )?;

    // For non-latest, filter the results to matching versions
    if !version.is_latest() {
        packages.retain(|package| {
            package.java_version == full_version || package.java_version == short_version
        });
    }

    // Find a package with our requested archive types
    let package = match find_package(&packages) {
        Some(package) => package,
        None => {
            return Err(plugin_err!(
                "No Java package available for version <hash>{full_version}</hash>. Using parameters: <mutedlight>distribution={} package={} release={} os={} arch={}</mutedlight>",
                config.distribution,
                config.package_type,
                config.release_type,
                env.os,
                env.arch
            ));
        }
    };

    // Then fetch download information
    let info = fetch_package_info(&config, &package.id)?;

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(match config.distribution {
            // Double nested on macos: openlogic-openjdk-x.x.x-mac-x64/jdk-x.x.x
            Distribution::Openlogic if env.os.is_mac() => "*/*".into(),
            // Nested in jdk dir: jdk-x.x.x
            _ => "*".into(),
        }),
        checksum: if info.is_checksum_supported_by_proto() {
            Some(Checksum::from_str(&format!(
                "{}:{}",
                info.checksum_type, info.checksum
            ))?)
        } else {
            None
        },
        checksum_url: if info.checksum_uri.is_empty() {
            None
        } else {
            Some(info.checksum_uri)
        },
        download_name: Some(info.filename),
        download_url: info.direct_download_uri,
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let config = get_tool_config::<JavaToolConfig>()?;

    // Liberica returns a flat folder structure, and does not use
    // the macOS bundle folder structure like other distros
    let bin_dir = if env.os.is_mac() && config.distribution != Distribution::Liberica {
        "Contents/Home/bin"
    } else {
        "bin"
    };

    let mut exes = HashMap::from_iter([(
        "java".into(),
        ExecutableConfig::new_primary(format!("{bin_dir}/{}", env.os.get_exe_name("java"))),
    )]);

    if config.package_type == PackageType::Jdk {
        exes.extend([
            (
                "javac".into(),
                ExecutableConfig::new(format!("{bin_dir}/{}", env.os.get_exe_name("javac"))),
            ),
            (
                "jar".into(),
                ExecutableConfig::new(format!("{bin_dir}/{}", env.os.get_exe_name("jar"))),
            ),
        ]);
    }

    Ok(Json(LocateExecutablesOutput {
        exes,
        exes_dirs: vec![PathBuf::from(bin_dir)],
        globals_lookup_dirs: vec!["$JAVA_HOME/bin".into()],
        ..LocateExecutablesOutput::default()
    }))
}
