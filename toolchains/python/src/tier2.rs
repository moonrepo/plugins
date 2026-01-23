use crate::config::*;
use crate::managers::*;
use crate::pyproject_toml::PyProjectTomlWithTools;
use extism_pdk::*;
use moon_pdk::{
    load_project_toolchain_config, load_toolchain_config, locate_root, locate_root_many,
    locate_root_many_with_check, parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use std::path::PathBuf;

fn gather_shared_paths(
    config: &PythonToolchainConfig,
    context: &MoonContext,
    project: &ProjectFragment,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    let current_dir = context.get_project_root(project);

    if let Some(venv_parent) = locate_root(&current_dir, &config.venv_name)
        && let Some(venv_root) = venv_parent.join(&config.venv_name).real_path()
    {
        paths.push(venv_root.join("Scripts"));
        paths.push(venv_root.join("bin"));
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendTaskCommandOutput::default();

    gather_shared_paths(&config, &input.context, &input.project, &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendTaskScriptOutput::default();

    gather_shared_paths(&config, &input.context, &input.project, &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn define_requirements(
    Json(input): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = DefineRequirementsOutput::default();

    if let Some(package_manager) = config.package_manager {
        output.requires.push(format!("unstable_{package_manager}"));
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = LocateDependenciesRootOutput::default();

    let Some(package_manager) = config.package_manager else {
        return Ok(Json(output));
    };

    let manifest_names = match package_manager {
        PythonPackageManager::Pip | PythonPackageManager::UvPip => {
            vec!["pyproject.toml", "requirements.in"]
        }
        PythonPackageManager::Uv => vec!["pyproject.toml"],
    };

    let lock_names = match package_manager {
        PythonPackageManager::Pip | PythonPackageManager::UvPip => {
            vec!["pylock.toml", "requirements.txt"]
        }
        PythonPackageManager::Uv => vec!["uv.lock"],
    };

    // First attempt: find lock files
    if let Some(root) = locate_root_many(&input.starting_dir, &lock_names) {
        output.root = root.virtual_path();
        output.members = PyProjectTomlWithTools::load(root.join("pyproject.toml"))?
            .extract_members(package_manager)?;
    }

    // Second attempt: find workspace-compatible manifest files
    if output.root.is_none() {
        locate_root_many_with_check(&input.starting_dir, &manifest_names, |root| {
            let manifest = PyProjectTomlWithTools::load(root.join("pyproject.toml"))?;
            let mut found = false;

            if manifest.tool.is_some()
                && let Some(members) = manifest.extract_members(package_manager)?
            {
                output.root = root.virtual_path();
                output.members = Some(members);
                found = true;
            }

            Ok(found)
        })?;
    }

    // Last attempt: find a manifest file (project only)
    if output.root.is_none()
        && let Some(root) = locate_root_many(&input.starting_dir, &manifest_names)
    {
        output.root = root.virtual_path();
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = InstallDependenciesOutput::default();

    let Some(package_manager) = config.package_manager else {
        return Ok(Json(output));
    };

    let mut include_reqs_and_constraints = false;
    let fallback_uv_args: Vec<String> = vec![
        "--no-managed-python".into(),
        "--no-python-downloads".into(),
        "--no-progress".into(),
    ];

    let package_manager_config: SharedPackageManagerConfig = match &input.project {
        Some(project) => load_project_toolchain_config(&project.id, package_manager.to_string())?,
        None => load_toolchain_config(package_manager.to_string())?,
    };

    // Install
    let mut command = match package_manager {
        PythonPackageManager::Pip => {
            include_reqs_and_constraints = true;

            ExecCommandInput::new("python", ["-m", "pip", "install"])
        }
        // PythonPackageManager::Poetry => ExecCommandInput::new("poetry", ["sync"]),
        PythonPackageManager::Uv => {
            let mut cmd = ExecCommandInput::new("uv", ["sync"]);

            for package_name in input.packages {
                cmd.args.push("--package".into());
                cmd.args.push(package_name);
            }

            cmd
        }
        PythonPackageManager::UvPip => {
            include_reqs_and_constraints = true;

            ExecCommandInput::new("uv", ["pip", "install"])
        }
    };

    if include_reqs_and_constraints {
        if input.root.join("requirements.txt").exists() {
            command.args.push("-r".into());
            command.args.push("requirements.txt".into());
        } else if input.root.join("requirements.in").exists() {
            command.args.push("-r".into());
            command.args.push("requirements.in".into());
        }

        if input.root.join("constraints.txt").exists() {
            command.args.push("-c".into());
            command.args.push("constraints.txt".into());
        }
    }

    if package_manager_config.install_args.is_empty() && package_manager.is_uv_based() {
        command.args.extend(fallback_uv_args);
    } else {
        command.args.extend(package_manager_config.install_args);
    }

    command.cwd = Some(input.root.clone());

    output.install_command = Some(command.into());

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();

    match input.path.file_name().and_then(|name| name.to_str()) {
        Some("pylock.toml") => parse_pylock_toml(&input.path, &mut output)?,
        Some("requirements.txt") => parse_requirements_txt(&input.path, &mut output)?,
        Some("uv.lock") => parse_uv_lock(&input.path, &mut output)?,
        _ => {}
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();

    match input.path.file_name().and_then(|name| name.to_str()) {
        Some("pyproject.toml") => parse_pyproject_toml(&input.path, &mut output)?,
        Some("requirements.in") => parse_requirements_in(&input.path, &mut output)?,
        _ => {}
    };

    Ok(Json(output))
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = SetupEnvironmentOutput::default();

    let Some(package_manager) = config.package_manager else {
        return Ok(Json(output));
    };

    let fallback_uv_args: Vec<String> = vec![
        "--no-managed-python".into(),
        "--no-python-downloads".into(),
        "--no-progress".into(),
    ];

    let package_manager_config: SharedPackageManagerConfig = match &input.project {
        Some(project) => load_project_toolchain_config(&project.id, package_manager.to_string())?,
        None => load_toolchain_config(package_manager.to_string())?,
    };

    let mut command = match package_manager {
        PythonPackageManager::Pip => {
            ExecCommandInput::new("python", ["-m", "venv", &config.venv_name])
        }
        PythonPackageManager::Uv | PythonPackageManager::UvPip => {
            ExecCommandInput::new("uv", ["venv", &config.venv_name])
        }
    };

    if package_manager_config.venv_args.is_empty()
        && config.version.is_some()
        && package_manager.is_uv_based()
    {
        command.args.extend(fallback_uv_args);
    } else {
        command.args.extend(package_manager_config.venv_args);
    }

    command.cwd = Some(input.root.clone());

    output.commands.push(command.into());

    Ok(Json(output))
}
