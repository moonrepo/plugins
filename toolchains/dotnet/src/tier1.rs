use crate::config::DotnetToolchainConfig;
use extism_pdk::*;
use moon_config::LanguageType;
use moon_pdk_api::*;
use schematic::SchemaBuilder;
use starbase_utils::fs;
use toolchain_common::enable_tracing;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: ".NET".into(),
        description: Some(
            "Provides .NET SDK project-graph extraction, dependency install (dotnet restore), and Docker support for SDK-style C#/F#/VB projects.".into(),
        ),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        language: Some(LanguageType::CSharp),
        exe_names: vec!["dotnet".into()],
        config_file_globs: vec![
            "*.{csproj,fsproj,vbproj}".into(),
            "*.{sln,slnx}".into(),
            "global.json".into(),
            "Directory.Build.props".into(),
            "Directory.Build.targets".into(),
            "Directory.Build.rsp".into(),
            "Directory.Packages.props".into(),
            // NuGet accepts any casing; cover the ones seen in the wild so
            // case-sensitive filesystems still match.
            "{nuget,NuGet}.{config,Config}".into(),
            // `packages.<project>.lock.json` via NuGetLockFilePath; the
            // default name lives in lock_file_names (exact match only there).
            "packages.*.lock.json".into(),
        ],
        // Project files (*.csproj) have variable names, which moon's
        // literal-name manifest matching cannot express; their detection is
        // covered by config_file_globs instead. Directory.Packages.props is
        // the one fixed-name .NET manifest: registering it makes CPM version
        // bumps re-trigger dependency installs via parse_manifest.
        manifest_file_names: vec!["Directory.Packages.props".into()],
        lock_file_names: vec!["packages.lock.json".into()],
        // NuGet uses a global package cache, not an in-repo vendor dir.
        vendor_dir_name: None,
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<DotnetToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    // There is nothing to prompt for: every setting has a working default, and
    // the SDK version is read from `global.json` rather than configured here.
    Ok(Json(InitializeToolchainOutput::default()))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(_): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    Ok(Json(DefineDockerMetadataOutput {
        // Intentionally not derived from the configured `version`: doing so
        // would mean reading toolchain config here to pick an image tag, which
        // is a product decision rather than a default. `version` is honoured
        // where it matters — tier 3 installs exactly that SDK.
        default_image: Some("mcr.microsoft.com/dotnet/sdk:latest".into()),
        scaffold_globs: vec![
            "**/*.{csproj,fsproj,vbproj}".into(),
            "**/*.{sln,slnx}".into(),
            "**/*.props".into(),
            "**/*.targets".into(),
            "**/Directory.Build.rsp".into(),
            "**/{nuget,NuGet}.{config,Config}".into(),
            "**/packages.lock.json".into(),
            "**/packages.*.lock.json".into(),
            "global.json".into(),
            // bin/obj contain generated *.props (obj/*.nuget.g.props) and
            // must never end up in the restore layer.
            "!**/bin/**".into(),
            "!**/obj/**".into(),
        ],
    }))
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let mut output = PruneDockerOutput::default();

    let mut roots = vec![input.root.clone()];

    for project in &input.projects {
        roots.push(input.context.get_project_root(project));
    }

    for root in roots {
        for dir_name in ["bin", "obj"] {
            let dir = root.join(dir_name);

            if dir.exists() {
                fs::remove_dir_all(&dir)?;

                if let Some(file) = dir.virtual_path() {
                    output.changed_files.push(file);
                }
            }
        }
    }

    Ok(Json(output))
}
