use crate::config::*;
use crate::managers::*;
use crate::pyproject_toml::{PyProjectToml, PyProjectTomlWithTools, normalize_distribution_name};
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk::{
    get_host_env_var, get_host_environment, load_project_toolchain_config, load_toolchain_config,
    locate_root, locate_root_many, locate_root_many_with_check, parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use pep508_rs::Requirement;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let mut output = ExtendProjectGraphOutput::default();

    // First pass, gather all packages and their manifests.
    let mut packages = BTreeMap::default();

    for (id, source) in input.project_sources {
        let project_root = input.context.get_project_root_from_source(&source);
        let manifest_path = project_root.join("pyproject.toml");

        if manifest_path.exists() {
            let mut manifest = PyProjectToml::load(manifest_path)?;

            // Remove fields we don't need to avoid eating a ton of memory
            manifest.build_system = None;
            manifest.dependency_groups = None;

            // We need to track all packages, even those without a name
            if let Some(project) = &mut manifest.project {
                project.description = None;
                project.authors = None;
                project.maintainers = None;
                project.keywords = None;
                project.classifiers = None;

                packages.insert(normalize_distribution_name(&project.name), (id, manifest));
            }
        }
    }

    // Second pass, extract packages and their relationships
    for (id, manifest) in packages.values() {
        let mut project_output = ExtendProjectOutput::default();

        let mut extract_implicit_deps =
            |reqs: &[Requirement], scope: DependencyScope| -> AnyResult<()> {
                for req in reqs {
                    let req_label = req.name.as_ref().to_owned();
                    let req_name = normalize_distribution_name(req.name.as_ref());

                    if req.version_or_url.is_none()
                        && req.origin.is_none()
                        && let Some((dep_id, _)) = packages.get(&req_name)
                    {
                        project_output.dependencies.push(ProjectDependency {
                            id: dep_id.to_owned(),
                            scope,
                            via: Some(format!("requirement {req_label}")),
                        });
                    }
                }

                Ok(())
            };

        if let Some(project) = &manifest.project {
            project_output.alias = Some(project.name.clone());

            if let Some(deps) = &project.dependencies {
                extract_implicit_deps(deps, DependencyScope::Production)?;
            }

            if let Some(deps) = &project.optional_dependencies {
                for reqs in deps.values() {
                    extract_implicit_deps(reqs, DependencyScope::Production)?;
                }
            }
        }

        output
            .extended_projects
            .insert(id.to_owned(), project_output);

        if let Some(file) = manifest.path.virtual_path() {
            output.input_files.push(file);
        }
    }

    Ok(Json(output))
}

fn gather_shared_paths(
    config: &PythonToolchainConfig,
    current_dir: &VirtualPath,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(venv_parent) = locate_root(current_dir, &config.venv_name)
        && let Some(venv_root) = venv_parent.join(&config.venv_name).real_path()
    {
        paths.push(venv_root.join("Scripts"));
        paths.push(venv_root.join("bin"));
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_command(
    Json(input): Json<ExtendCommandInput>,
) -> FnResult<Json<ExtendCommandOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendTaskCommandOutput::default();

    gather_shared_paths(&config, &input.current_dir, &mut output.paths)?;

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
    let mut package_manager_id = "pip";

    // Install
    let mut command = match package_manager {
        PythonPackageManager::Pip => {
            include_reqs_and_constraints = true;

            ExecCommandInput::new("python", ["-m", "pip", "install"])
        }
        PythonPackageManager::Uv => {
            package_manager_id = "uv";

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

    let package_manager_config: SharedPackageManagerConfig = match &input.project {
        Some(project) => load_project_toolchain_config(&project.id, package_manager_id)?,
        None => load_toolchain_config(package_manager_id)?,
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

    if package_manager_config.install_args.is_empty()
        && matches!(package_manager, PythonPackageManager::Uv)
    {
        command.args.extend(get_uv_fallback_args(&config));
    } else {
        command.args.extend(package_manager_config.install_args);
    }

    command.cwd = Some(input.root.clone());

    // Activate the venv by modifying PATH
    let mut activation_paths: Vec<PathBuf> = Vec::new();

    if let Some(project) = &input.project {
        gather_shared_paths(
            &config,
            &input.context.get_project_root(project),
            &mut activation_paths,
        )?;
    }

    let host = get_host_environment()?;
    let sep = if host.os.is_windows() { ';' } else { ':' };

    let mut prefix = String::new();
    for p in activation_paths {
        if !prefix.is_empty() {
            prefix.push(sep);
        }
        prefix.push_str(&p.to_string_lossy());
    }

    let path = get_host_env_var("PATH")?.unwrap_or_default();

    command.env.insert(
        "PATH".into(),
        if prefix.is_empty() {
            path
        } else {
            format!("{prefix}{sep}{path}")
        },
    );

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

    let (mut command, package_manager_id) = match package_manager {
        PythonPackageManager::Pip => (
            ExecCommandInput::new("python", ["-m", "venv", &config.venv_name]),
            "pip",
        ),
        PythonPackageManager::Uv | PythonPackageManager::UvPip => (
            ExecCommandInput::new("uv", ["venv", &config.venv_name]),
            "uv",
        ),
    };

    let package_manager_config: SharedPackageManagerConfig = match &input.project {
        Some(project) => load_project_toolchain_config(&project.id, package_manager_id)?,
        None => load_toolchain_config(package_manager_id)?,
    };

    if package_manager_config.venv_args.is_empty()
        && matches!(
            package_manager,
            PythonPackageManager::Uv | PythonPackageManager::UvPip
        )
    {
        command.args.extend(get_uv_fallback_args(&config));
    } else {
        command.args.extend(package_manager_config.venv_args);
    }

    command.cwd = Some(input.root.clone());

    output.commands.push(command.into());

    Ok(Json(output))
}

fn get_uv_fallback_args(config: &PythonToolchainConfig) -> Vec<String> {
    let mut args = vec![];

    if config.version.is_some() {
        args.push("--no-managed-python".into());
        args.push("--no-python-downloads".into());
    }

    args.push("--no-progress".into());
    args
}
