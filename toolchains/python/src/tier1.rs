use crate::config::{PythonPackageManager, PythonToolchainConfig};
use crate::pyproject_toml::PyProjectToml;
use extism_pdk::*;
use moon_config::{LanguageType, UnresolvedVersionSpec};
use moon_pdk::parse_toolchain_config_schema;
use moon_pdk_api::*;
use python_tool::to_python_version;
use schematic::SchemaBuilder;
use starbase_utils::json::JsonValue;
use starbase_utils::{fs, glob};
use std::path::PathBuf;
use toolchain_common::{enable_tracing, get_docker_image_tag};

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolchainOutput {
        name: "Python".into(),
        description: Some(
            "Installs dependencies and provides sync operations that keep project's in a healthy state."
                .into(),
        ),
        language: Some(LanguageType::Python),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        // For project detection
        config_file_globs: vec![
            "poetry.*".into(),
            "uv.*".into(),
            ".python-version".into(),
            // Read by proto
            ".python-versions".into(),
            ".venv".into(),
        ],
        manifest_file_names: vec![
            "pyproject.toml".into(),
            "requirements.in".into(),
            // pip
            "Pipfile".into(),
            // poetry
            "poetry.toml".into(),
            // uv
            "uv.toml".into(),
        ],
        lock_file_names: vec![
            // uv
            "uv.lock".into(),
            // poetry
            "poetry.lock".into(),
            // pip
            "pylock.toml".into(),
            "requirements.txt".into(),
            "Pipfile.lock".into(),
            "constraints.txt".into(),
        ],
        // For task detection
        exe_names: vec![
            "python".into(),
            "python2".into(),
            "python-2".into(),
            "python3".into(),
            "python-3".into(),
            // pip
            "pip".into(),
            "pip3".into(),
            // poetry
            "poetry".into(),
            // uv
            "uv".into(),
            "uvx".into(),
        ],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(input): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    let mut output = InitializeToolchainOutput {
        config_url: Some("https://moonrepo.dev/docs/config/toolchain#python".into()),
        docs_url: Some("https://moonrepo.dev/docs/config/toolchain#python".into()),
        ..Default::default()
    };

    if let Some(package_manager) = detect_package_manager(&input.context.working_dir)? {
        output.default_settings.insert(
            "packageManager".into(),
            JsonValue::String(package_manager.to_string()),
        );
    } else {
        output.prompts.push(SettingPrompt::new(
            "packageManager",
            "Package manager to install dependencies with?",
            PromptType::Select {
                default_index: 0,
                options: vec![
                    JsonValue::String("pip".into()),
                    JsonValue::String("poetry".into()),
                    JsonValue::String("uv".into()),
                    JsonValue::String("uv-pip".into()),
                ],
            },
        ));
    }

    Ok(Json(output))
}

fn detect_package_manager(root: &VirtualPath) -> AnyResult<Option<PythonPackageManager>> {
    if root.join("uv.toml").exists() || root.join("uv.lock").exists() {
        return Ok(Some(PythonPackageManager::Uv));
    } else if root.join("poetry.toml").exists() || root.join("poetry.lock").exists() {
        return Ok(Some(PythonPackageManager::Poetry));
    }

    let manifest = PyProjectToml::load(root.join("pyproject.toml"))?;

    if manifest.get_tool_uv().is_some() {
        return Ok(Some(PythonPackageManager::Uv));
    } else if manifest.get_tool_poetry().is_some() {
        return Ok(Some(PythonPackageManager::Poetry));
    }

    // Pipfiles are pipenv's, which pip can't install
    if root.join("requirements.txt").exists() {
        return Ok(Some(PythonPackageManager::Pip));
    }

    Ok(None)
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: SchemaBuilder::build_root::<PythonToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn define_docker_metadata(
    Json(input): Json<DefineDockerMetadataInput>,
) -> FnResult<Json<DefineDockerMetadataOutput>> {
    // With the schema, so `venvName` gets its default
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;

    let tag = match &config.version {
        // Docker Hub uses CPython's form for pre-releases, like `3.15.0rc2`
        Some(UnresolvedVersionSpec::Version(version)) => to_python_version(version),
        // Aliases like `stable` aren't tags
        version => get_docker_image_tag(version.as_ref(), |_| None),
    };

    Ok(Json(DefineDockerMetadataOutput {
        default_image: Some(format!("python:{tag}")),
        scaffold_globs: vec![
            // setuptools' build config, which `docker setup` needs to install the project
            "setup.py".into(),
            "setup.cfg".into(),
            // The sources phase copies every file, and a venv from the host would
            // be laid over the one `docker setup` created in the image
            format!("!{}/**/*", config.venv_name),
            format!("!**/{}/**/*", config.venv_name),
        ],
    }))
}

#[plugin_fn]
pub fn scaffold_docker(
    Json(input): Json<ScaffoldDockerInput>,
) -> FnResult<Json<ScaffoldDockerOutput>> {
    let mut output = ScaffoldDockerOutput::default();

    // `docker setup` installs local packages from the configs skeleton, which only
    // has manifests and lockfiles, and build backends fail without the package's
    // modules, or the files that its manifest points to. So stub them, and the
    // sources phase lays the real files over the stubs (D5).
    if input.phase != ScaffoldDockerPhase::Configs || input.project.is_none() {
        return Ok(Json(output));
    }

    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;

    for path in get_stub_paths(&input.input_dir, &config.venv_name)? {
        let file = input.output_dir.join(path);

        // Keep a file that was copied, like from `configsPhaseGlobs`
        if !file.exists() {
            fs::write_file(&file, "")?;
            output.copied_files.push(file);
        }
    }

    Ok(Json(output))
}

/// The packages' `__init__.py` files, and the files that `pyproject.toml`
/// points to, relative to the project root.
fn get_stub_paths(input_dir: &VirtualPath, venv_name: &str) -> AnyResult<Vec<PathBuf>> {
    let venv_globs = [
        format!("!{venv_name}/**/*"),
        format!("!**/{venv_name}/**/*"),
    ];
    let walk = |globs: Vec<String>| -> AnyResult<Vec<PathBuf>> {
        Ok(
            glob::walk_files(input_dir, globs.iter().chain(&venv_globs))?
                .into_iter()
                .filter_map(|file| file.strip_prefix(input_dir).ok().map(PathBuf::from))
                .collect(),
        )
    };

    let mut paths = walk(vec!["**/__init__.py".into()])?;
    let (files, license_globs) =
        PyProjectToml::load(input_dir.join("pyproject.toml"))?.get_build_files();

    for file in files {
        if input_dir.join(&file).is_file() {
            paths.push(PathBuf::from(file));
        }
    }

    if !license_globs.is_empty() {
        // ADR-0008.6
        match walk(license_globs) {
            Ok(files) => paths.extend(files),
            Err(error) => debug!("Skipping the license files of {input_dir}: {error}"),
        };
    }

    paths.sort();
    paths.dedup();

    Ok(paths)
}

#[plugin_fn]
pub fn prune_docker(Json(input): Json<PruneDockerInput>) -> FnResult<Json<PruneDockerOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = PruneDockerOutput::default();

    if input.docker_config.delete_vendor_directories {
        for dep in input.project_dependencies {
            let dep_root = input.context.get_project_root(&dep);
            let venv_dir = dep_root.join(&config.venv_name);

            if venv_dir.exists() {
                fs::remove_dir_all(&venv_dir)?;

                output.changed_files.push(venv_dir);
            }
        }
    }

    Ok(Json(output))
}
