use crate::config::*;
use extism_pdk::*;
use moon_pdk::{
    get_host_environment, load_project_toolchain_config, load_toolchain_config, locate_root,
    parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use starbase_utils::fs;
use std::path::PathBuf;

// #[plugin_fn]
// pub fn setup_environment(
//     Json(input): Json<SetupEnvironmentInput>,
// ) -> FnResult<Json<SetupEnvironmentOutput>> {
//     let config = parse_toolchain_config::<NodeToolchainConfig>(input.toolchain_config)?;
//     let mut output = SetupEnvironmentOutput::default();

//     // Sync version manager
//     if let Some(version_manager) = config.sync_version_manager_config
//         && let Some(version) = config.version
//     {
//         let (op, file) = Operation::track("sync-version-manager", || {
//             let rc_path = input.root.join(match version_manager {
//                 NodeVersionManager::Nodenv => ".node-version",
//                 NodeVersionManager::Nvm => ".nvmrc",
//             });

//             fs::write_file(&rc_path, version.to_partial_string())?;

//             Ok(rc_path)
//         })?;

//         output.operations.push(op);
//         output.changed_files.extend(file.virtual_path());
//     }

//     Ok(Json(output))
// }

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
        output.requires.push(package_manager.to_string());
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let config =
        parse_toolchain_config_schema::<JavaScriptToolchainConfig>(input.toolchain_config)?;
    let mut output = LocateDependenciesRootOutput::default();

    let Some(package_manager) = config.package_manager else {
        return Ok(Json(output));
    };

    let manifest_names = vec!["pyproject.toml"];

    let lock_names = match package_manager {
        PythonPackageManager::Pip => vec!["requirements.txt", "pylock.toml"],
        PythonPackageManager::Uv => vec!["uv.lock"],
    };

    // First attempt: find lock files
    if let Some(root) = locate_root_many(&input.starting_dir, &lock_names) {
        output.root = root.virtual_path();
        output.members = extract_workspace_members_and_catalogs(package_manager, &root)?;
    }

    // Second attempt: find workspace-compatible manifest files
    if output.root.is_none() {
        locate_root_many_with_check(&input.starting_dir, &workspace_manifest_names, |root| {
            let mut found = false;

            if let Some(members) = extract_workspace_members_and_catalogs(package_manager, root)? {
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
        extract_workspace_members_and_catalogs(package_manager, &root)?;

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

    let mut fallback_install_args: Vec<String> = vec![];
    let package_manager_config: SharedPackageManagerConfig = match &input.project {
        Some(project) => load_project_toolchain_config(&project.id, package_manager.to_string())?,
        None => load_toolchain_config(package_manager.to_string())?,
    };

    // Install
    let mut command = match package_manager {
        PythonPackageManager::Pip => ExecCommandInput::new("python", ["-m", "pip", "install"]),
        // PythonPackageManager::Poetry => ExecCommandInput::new("poetry", ["sync"]),
        PythonPackageManager::Uv => {
            fallback_install_args.extend([
                "--no-managed-python".into(),
                "--no-python-downloads".into(),
                "--no-progress".into(),
            ]);

            let mut cmd = ExecCommandInput::new("uv", ["sync"]);

            for package_name in input.packages {
                cmd.args.push("--package".into());
                cmd.args.push(package_name);
            }

            cmd
        }
    };

    if package_manager_config.install_args.is_empty() {
        command.args.extend(fallback_install_args);
    } else {
        command.args.extend(package_manager_config.install_args);
    }

    command.cwd = Some(input.root.clone());

    output.install_command = Some(command.into());

    Ok(Json(output))
}
