use crate::config::{Distribution, JavaToolConfig, PackageType};
use crate::foojay::{fetch_package_info, fetch_packages, find_package};
use crate::java::JavaContext;
use crate::version::from_java_version;
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use tool_common::enable_tracing;

#[plugin_fn]
pub fn register_tool(Json(input): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: "Java".into(),
        type_of: PluginType::Language,
        inventory_options: ToolInventoryOptions {
            override_dir_name: Some(if input.id == "jre" {
                "jre".into()
            } else {
                "jdk".into()
            }),
            ..Default::default()
        },
        minimum_proto_version: Some(Version::new(0, 59, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        unstable: Switch::Toggle(true),
        ..Default::default()
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
    let is_sdkman = input.file == ".sdkmanrc";

    fn normalize_sdkman_version(value: &str) -> AnyResult<(Distribution, &str)> {
        let value = value.trim();

        if let Some((version, vendor)) = value.rsplit_once('-')
            && vendor.chars().all(|c| c.is_ascii_alphanumeric())
        {
            Ok((Distribution::parse(vendor)?, version))
        } else {
            Ok((Distribution::default(), value))
        }
    }

    if is_sdkman || input.file == ".java-version" {
        for line in input.content.lines() {
            let line = line.trim();

            if line.is_empty() || is_sdkman && !line.starts_with("java=") {
                continue;
            }

            if is_sdkman {
                let (dist, value) =
                    normalize_sdkman_version(line.strip_prefix("java=").unwrap_or(line))?;

                version = Some(UnresolvedVersionSpec::parse(format!("{dist}-{value}"))?);
            } else {
                // Lines may already contain a distribution scope (temurin-21),
                // or be an alias (latest), so only inject the default scope
                // into unscoped versions and requirements
                let mut spec = UnresolvedVersionSpec::parse(line)?;

                if spec.get_scope().is_none() {
                    spec.set_scope(Distribution::default().to_string());
                }

                version = Some(spec);
            }

            break;
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(input): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let env = get_host_environment()?;
    let config = get_tool_config::<JavaToolConfig>()?;
    let java = JavaContext::detect_from_unresolved(&input.initial)?;

    // Scope each version with its distribution, as scoped requirements
    // (created by `resolve_version`) only match versions of the same scope
    let versions = fetch_packages(&env, &config, &java)?
        .into_iter()
        .filter_map(|package| {
            package
                .distribution
                .as_ref()
                .map(|dist| format!("{}-{}", dist, from_java_version(&package.java_version)))
        })
        .collect::<HashSet<_>>();

    let mut output = LoadVersionsOutput::from(versions.into_iter().collect())?;

    // Every Java version carries build metadata (21.0.11+10), which
    // `from` excludes when computing the latest version, so compute
    // it ourselves from the stable (non pre-release) versions

    let latest = output
        .versions
        .iter()
        .filter(|spec| {
            spec.get_scope()
                .as_ref()
                .and_then(|scope| Distribution::parse(scope).ok())
                .is_some_and(|scope| scope == java.distribution)
                && spec
                    .as_version()
                    .is_some_and(|version| version.prerelease.is_none())
        })
        .max()
        .cloned();

    if let Some(latest) = latest {
        let latest = latest.to_unresolved_spec();

        output.aliases.insert("latest".into(), latest.clone());
        output.latest = Some(latest);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();
    let mut initial = input.initial.clone();

    // If the version is missing a vendor, inject the default one,
    // otherwise validate the vendor that is provided. Only requirements
    // and versions support scopes, so aliases pass through untouched.
    match initial.get_scope() {
        Some(scope) => {
            Distribution::parse(scope)?;
        }
        None if matches!(
            initial,
            UnresolvedVersionSpec::Requirement(_) | UnresolvedVersionSpec::Version(_)
        ) =>
        {
            initial.set_scope(Distribution::default().to_string());
            output.candidate = Some(initial);
        }
        None => {}
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let base_version = &input.context.version;

    if base_version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: "Java".into()
        }));
    }

    let env = get_host_environment()?;
    let config = get_tool_config::<JavaToolConfig>()?;
    let java = JavaContext::detect(base_version)?;

    // Load all matching packages
    let mut packages = fetch_packages(&env, &config, &java)?;

    // For non-latest, filter the results to matching versions. Also gate on
    // the distribution, as multiple distributions share identical java
    // versions (temurin and zulu both publish 21.0.11+10, for example)
    if !java.spec.is_latest() {
        packages.retain(|package| {
            package.distribution.as_ref() == Some(&java.distribution)
                && (package.java_version == java.full_version
                    || package.java_version == java.short_version
                    || from_java_version(&package.java_version) == java.full_version)
        });
    }

    // Find a package with our requested archive types
    let package = match find_package(&packages, &env) {
        Some(package) => package,
        None => {
            return Err(plugin_err!(
                "No Java package available for version <hash>{}</hash>. Using parameters: <mutedlight>distribution={} package={} release={} os={} arch={}</mutedlight>",
                java.full_version,
                java.distribution,
                java.package,
                config.release_type,
                env.os,
                env.arch
            ));
        }
    };

    // Then fetch download information
    let info = fetch_package_info(&config, &package.id)?;

    Ok(Json(DownloadPrebuiltOutput {
        archive_prefix: Some(match java.distribution {
            // Double nested on macos: openlogic-openjdk-x.x.x-mac-x64/jdk-x.x.x
            Distribution::OpenLogic if env.os.is_mac() => "*/*".into(),
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
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let java = JavaContext::detect(&input.context.version)?;

    let home_dir = get_home_dir(&input.context.tool_dir);
    let bin_dir = if home_dir.ends_with("Contents/Home") {
        "Contents/Home/bin"
    } else {
        "bin"
    };

    // Both the JDK and JRE provide the java runtime
    let mut exes = HashMap::from_iter([(
        "java".into(),
        ExecutableConfig::new_primary(format!("{bin_dir}/{}", env.os.get_exe_name("java"))),
    )]);

    // While development tools only exist in the JDK
    if java.package == PackageType::Jdk {
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
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn activate_environment(
    Json(input): Json<ActivateEnvironmentInput>,
) -> FnResult<Json<ActivateEnvironmentOutput>> {
    let mut output = ActivateEnvironmentOutput::default();
    let home_dir = get_home_dir(&input.context.tool_dir);

    if let Some(home) = home_dir.real_path_string() {
        output.env.insert("JAVA_HOME".into(), home);
    }

    Ok(Json(output))
}

fn get_home_dir(base: &VirtualPath) -> VirtualPath {
    let home_dir = base.join("Contents").join("Home");

    if home_dir.exists() {
        home_dir
    } else {
        base.to_owned()
    }
}
