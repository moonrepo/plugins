use crate::config::JavaToolConfig;
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tool_common::enable_tracing;

static NAME: &str = "Java";

#[derive(Debug, Deserialize)]
#[serde(default)]
struct FoojayPackageResponse {
    result: Vec<FoojayPackage>,
}

impl Default for FoojayPackageResponse {
    fn default() -> Self {
        Self { result: vec![] }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct FoojayPackageInfoResponse {
    result: Vec<FoojayPackageInfo>,
}

impl Default for FoojayPackageInfoResponse {
    fn default() -> Self {
        Self { result: vec![] }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct FoojayLinks {
    pkg_download_redirect: String,
    pkg_info_uri: String,
}

impl Default for FoojayLinks {
    fn default() -> Self {
        Self {
            pkg_download_redirect: String::new(),
            pkg_info_uri: String::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct FoojayPackage {
    archive_type: String,
    architecture: String,
    directly_downloadable: bool,
    distribution: String,
    distribution_version: String,
    feature: Vec<String>,
    filename: String,
    id: String,
    java_version: String,
    lib_c_type: String,
    links: FoojayLinks,
    operating_system: String,
    package_type: String,
    release_status: String,
}

impl Default for FoojayPackage {
    fn default() -> Self {
        Self {
            archive_type: String::new(),
            architecture: String::new(),
            directly_downloadable: false,
            distribution: String::new(),
            distribution_version: String::new(),
            feature: vec![],
            filename: String::new(),
            id: String::new(),
            java_version: String::new(),
            lib_c_type: String::new(),
            links: FoojayLinks::default(),
            operating_system: String::new(),
            package_type: String::new(),
            release_status: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct FoojayPackageInfo {
    checksum: Option<String>,
    checksum_type: Option<String>,
    direct_download_uri: String,
    filename: String,
}

impl Default for FoojayPackageInfo {
    fn default() -> Self {
        Self {
            checksum: None,
            checksum_type: None,
            direct_download_uri: String::new(),
            filename: String::new(),
        }
    }
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
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

    if input.file == ".sdkmanrc" {
        for line in input.content.lines() {
            let line = line.trim();

            if let Some(value) = line.strip_prefix("java=") {
                version = Some(UnresolvedVersionSpec::parse(normalize_sdkman_version(
                    value,
                ))?);
                break;
            }
        }
    } else if input.file == ".java-version" {
        for line in input.content.lines() {
            let line = line.trim();

            if !line.is_empty() {
                version = Some(UnresolvedVersionSpec::parse(normalize_sdkman_version(
                    line,
                ))?);
                break;
            }
        }
    }

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let env = get_host_environment()?;
    let config = get_tool_config::<JavaToolConfig>()?;
    let versions = load_packages(&env, &config, None)?
        .into_iter()
        .filter(|package| is_matching_package(&env, package, &config))
        .filter_map(|package| {
            let version = to_proto_version(&package);

            VersionSpec::parse(&version).ok()
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(Json(LoadVersionsOutput::from_versions(versions)))
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
            HostOS::Linux => [HostArch::X64, HostArch::Arm64, HostArch::Arm],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64, HostArch::Arm64],
        ],
    )?;

    let config = get_tool_config::<JavaToolConfig>()?;
    let version = &input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: NAME.into()
        }));
    }

    let package = select_package(&env, &config, version)?;
    let package_info = load_package_info(&package)?;
    let download_url = if package_info.direct_download_uri.is_empty() {
        package.links.pkg_download_redirect.clone()
    } else {
        package_info.direct_download_uri.clone()
    };
    let download_name = if !package_info.filename.is_empty() {
        Some(package_info.filename.clone())
    } else if !package.filename.is_empty() {
        Some(package.filename.clone())
    } else {
        download_url
            .split('/')
            .next_back()
            .map(|name| name.to_owned())
    };

    Ok(Json(DownloadPrebuiltOutput {
        checksum: to_checksum(&package_info),
        download_name,
        download_url,
        ..DownloadPrebuiltOutput::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(input): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;
    let config = get_tool_config::<JavaToolConfig>()?;
    let install_dir = input
        .install_dir
        .real_path()
        .unwrap_or_else(|| input.install_dir.to_path_buf());
    let java_home = find_java_home(&install_dir, &env).unwrap_or_default();
    let bin_dir = java_home.join("bin");
    let java_exe = bin_dir.join(env.os.get_exe_name("java"));
    let javac_exe = bin_dir.join(env.os.get_exe_name("javac"));
    let jar_exe = bin_dir.join(env.os.get_exe_name("jar"));
    let java_home_var = if java_home.as_os_str().is_empty() {
        "$TOOL_DIR".to_owned()
    } else {
        format!("$TOOL_DIR/{}", java_home.display())
    };
    let shim_env_vars = Some(HashMap::from_iter([("JAVA_HOME".into(), java_home_var)]));

    let mut exes = HashMap::from_iter([(
        "java".into(),
        ExecutableConfig {
            shim_env_vars: shim_env_vars.clone(),
            ..ExecutableConfig::new_primary(java_exe.to_string_lossy())
        },
    )]);

    if config.image_type == "jdk" {
        exes.extend([
            (
                "javac".into(),
                ExecutableConfig {
                    shim_env_vars: shim_env_vars.clone(),
                    ..ExecutableConfig::new(javac_exe.to_string_lossy())
                },
            ),
            (
                "jar".into(),
                ExecutableConfig {
                    shim_env_vars,
                    ..ExecutableConfig::new(jar_exe.to_string_lossy())
                },
            ),
        ]);
    }

    Ok(Json(LocateExecutablesOutput {
        exes,
        exes_dirs: vec![bin_dir],
        globals_lookup_dirs: vec!["$JAVA_HOME/bin".into()],
        ..LocateExecutablesOutput::default()
    }))
}

fn load_packages(
    env: &HostEnvironment,
    config: &JavaToolConfig,
    version: Option<&VersionSpec>,
) -> FnResult<Vec<FoojayPackage>> {
    let mut url = format!(
        "{}/packages?distribution={}&package_type={}&release_status={}&operating_system={}&architecture={}&archive_type={}&latest=available",
        config.api_url.trim_end_matches('/'),
        query_value(&config.vendor),
        query_value(&config.image_type),
        query_value(&config.release_type),
        java_os(env)?,
        java_arch(env)?,
        archive_type(env),
    );

    if let Some(version) = version
        && !version.is_latest()
    {
        url.push_str("&version=");
        url.push_str(&query_value(&version.to_string()));
    }

    let response: FoojayPackageResponse = fetch_json(url)?;

    Ok(response.result)
}

fn load_package_info(package: &FoojayPackage) -> FnResult<FoojayPackageInfo> {
    let url = if !package.links.pkg_info_uri.is_empty() {
        package.links.pkg_info_uri.clone()
    } else {
        format!(
            "https://api.foojay.io/disco/v3.0/ids/{}",
            query_value(&package.id)
        )
    };
    let response: FoojayPackageInfoResponse = fetch_json(url)?;

    response
        .result
        .into_iter()
        .next()
        .ok_or_else(|| plugin_err!("Unable to load Java package information from Foojay."))
}

fn select_package(
    env: &HostEnvironment,
    config: &JavaToolConfig,
    version: &VersionSpec,
) -> FnResult<FoojayPackage> {
    let requested_version = version.to_string();
    let packages = load_packages(env, config, Some(version))?;
    let matches = packages
        .into_iter()
        .filter(|package| is_matching_package(env, package, config))
        .filter(|package| version.is_latest() || to_proto_version(package) == requested_version)
        .collect::<Vec<_>>();

    if version.is_latest() {
        matches
            .into_iter()
            .filter_map(|package| {
                VersionSpec::parse(&to_proto_version(&package))
                    .ok()
                    .map(|version| (version, package))
            })
            .max_by(|(a, _), (b, _)| a.cmp(b))
            .map(|(_, package)| package)
    } else {
        matches.into_iter().next()
    }
    .ok_or_else(|| {
        plugin_err!(
            "No Java pre-built available for version <hash>{}</hash> on <id>{}</id>/<id>{}</id> using vendor <id>{}</id> and image type <id>{}</id>. Prompt me for next steps.",
            version,
            java_os(env).unwrap_or("unknown"),
            java_arch(env).unwrap_or("unknown"),
            config.vendor,
            config.image_type,
        )
    })
}

fn is_matching_package(
    env: &HostEnvironment,
    package: &FoojayPackage,
    config: &JavaToolConfig,
) -> bool {
    package.directly_downloadable
        && package.distribution == config.vendor
        && package.package_type == config.image_type
        && package.release_status == config.release_type
        && package.operating_system == java_os(env).unwrap_or_default()
        && package.architecture == java_arch(env).unwrap_or_default()
        && package.archive_type == archive_type(env)
        && is_supported_libc(env, package)
        && package.feature.is_empty()
}

fn is_supported_libc(env: &HostEnvironment, package: &FoojayPackage) -> bool {
    if !env.os.is_linux() {
        return true;
    }

    package.lib_c_type.is_empty() || package.lib_c_type == java_libc(env)
}

fn to_proto_version(package: &FoojayPackage) -> String {
    let java_version = if package.java_version.is_empty() {
        &package.distribution_version
    } else {
        &package.java_version
    };

    if let Some(patch) = java_version.strip_prefix("1.8.0_") {
        return format!("8.0.{}", patch.split('-').next().unwrap_or(patch));
    }

    if let Some(patch) = java_version.strip_prefix("8u") {
        return format!("8.0.{}", patch.split(['+', '-']).next().unwrap_or(patch));
    }

    java_version
        .split_once('+')
        .map(|(version, _)| version)
        .unwrap_or(java_version)
        .to_owned()
}

fn to_checksum(package_info: &FoojayPackageInfo) -> Option<Checksum> {
    let checksum = package_info.checksum.as_deref()?;
    let checksum_type = package_info.checksum_type.as_deref().unwrap_or_default();

    if checksum_type.is_empty() {
        Checksum::from_str(checksum).ok()
    } else {
        Checksum::from_str(&format!("{checksum_type}:{checksum}")).ok()
    }
}

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

fn java_os(env: &HostEnvironment) -> FnResult<&'static str> {
    Ok(match env.os {
        HostOS::Linux => "linux",
        HostOS::MacOS => "macos",
        HostOS::Windows => "windows",
        _ => {
            return Err(plugin_err!(PluginError::UnsupportedOS {
                tool: NAME.into(),
                os: env.os.to_string(),
            }));
        }
    })
}

fn java_arch(env: &HostEnvironment) -> FnResult<&'static str> {
    Ok(match env.arch {
        HostArch::Arm => "arm",
        HostArch::Arm64 => "aarch64",
        HostArch::X64 => "x64",
        _ => {
            return Err(plugin_err!(PluginError::UnsupportedArch {
                tool: NAME.into(),
                arch: env.arch.to_string(),
            }));
        }
    })
}

fn java_libc(env: &HostEnvironment) -> &'static str {
    match env.libc {
        HostLibc::Musl => "musl",
        _ => "glibc",
    }
}

fn archive_type(env: &HostEnvironment) -> &'static str {
    match env.os {
        HostOS::Windows => "zip",
        _ => "tar.gz",
    }
}

fn query_value(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('+', "%2B")
        .replace(' ', "%20")
}

fn find_java_home(install_dir: &Path, env: &HostEnvironment) -> Option<PathBuf> {
    let exe_name = env.os.get_exe_name("java");

    for candidate in [
        PathBuf::new(),
        PathBuf::from("Contents/Home"),
        PathBuf::from("Home"),
    ] {
        if install_dir
            .join(&candidate)
            .join("bin")
            .join(&exe_name)
            .exists()
        {
            return Some(candidate);
        }
    }

    find_java_home_nested(install_dir, install_dir, &exe_name, 0)
}

fn find_java_home_nested(
    root_dir: &Path,
    current_dir: &Path,
    exe_name: &str,
    depth: usize,
) -> Option<PathBuf> {
    if depth > 4 {
        return None;
    }

    let entries = std::fs::read_dir(current_dir).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        if path.join("bin").join(exe_name).exists() {
            return path.strip_prefix(root_dir).ok().map(PathBuf::from);
        }

        if let Some(home) = find_java_home_nested(root_dir, &path, exe_name, depth + 1) {
            return Some(home);
        }
    }

    None
}
