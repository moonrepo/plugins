use crate::config::*;
use crate::managers::*;
use crate::pyproject_toml::{PyProjectToml, normalize_distribution_name, normalize_path};
use crate::venv::{is_venv_version_compatible, parse_pyvenv_cfg_version, to_uv_python_request};
use extism_pdk::*;
use moon_config::{DependencyScope, UnresolvedVersionSpec, Version};
use moon_pdk::VirtualPathExt;
use moon_pdk::{
    get_host_env_var, get_host_environment, load_project_toolchain_config, load_toolchain_config,
    locate_root, locate_root_many, locate_root_many_with_check, parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use pep508_rs::{Requirement, VersionOrUrl};
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

#[plugin_fn]
pub fn define_requirements(
    Json(input): Json<DefineRequirementsInput>,
) -> FnResult<Json<DefineRequirementsOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;

    Ok(Json(DefineRequirementsOutput {
        requires: match config.package_manager {
            // `uv pip` installs with the pip toolchain's args,
            // and creates the venv with the uv toolchain's
            Some(PythonPackageManager::UvPip) => {
                vec!["unstable_uv".into(), "unstable_pip".into()]
            }
            Some(package_manager) => vec![format!("unstable_{package_manager}")],
            None => vec![],
        },
        // We must ensure package managers are fully installed
        // before we setup the environment, otherwise commands fail
        for_setup_environment: true,
        // However, we don't want this when we setup this toolchain,
        // because the package managers depend on us, and it would
        // introduce a cycle
        for_setup_toolchain: false,
    }))
}

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let is_poetry = matches!(config.package_manager, Some(PythonPackageManager::Poetry));
    let is_uv = matches!(
        config.package_manager,
        Some(PythonPackageManager::Uv | PythonPackageManager::UvPip)
    );
    let mut output = ExtendProjectGraphOutput::default();
    let mut input_files = vec![];

    // First pass, gather all manifests, and the project for each package name and root
    let mut manifests = BTreeMap::default();
    let mut names = BTreeMap::default();
    let mut roots = BTreeMap::default();

    for (id, source) in input.project_sources {
        let project_root = input.context.get_project_root_from_source(&source);
        let manifest_path = project_root.join("pyproject.toml");

        if !manifest_path.exists() {
            continue;
        }

        let manifest = PyProjectToml::load(manifest_path)?;
        let name = manifest
            .project
            .as_ref()
            .and_then(|project| project.name.clone())
            .or_else(|| {
                // Poetry 1.x projects only have `[tool.poetry]`
                if is_poetry {
                    manifest
                        .get_tool_poetry()
                        .and_then(|poetry| poetry.name.clone())
                } else {
                    None
                }
            });

        // A manifest without a name isn't a package, like one that only configures tools
        let Some(name) = name else {
            continue;
        };

        names.insert(normalize_distribution_name(&name), id.clone());
        roots.insert(normalize_path(&project_root), id.clone());
        manifests.insert(id, (project_root, name, manifest));
    }

    // Second pass, extract packages and their relationships
    let mut workspace_roots = BTreeMap::default();

    for (id, (project_root, name, manifest)) in &manifests {
        let mut project_output = ExtendProjectOutput {
            alias: Some(name.to_owned()),
            ..Default::default()
        };

        // Members inherit the workspace root's sources, and their own take precedence.
        // Paths are relative to the manifest that declares them.
        let mut uv_sources = BTreeMap::default();
        let workspace_root = if is_uv {
            find_uv_workspace_root(project_root, &mut workspace_roots)?
        } else {
            None
        };

        if let Some((root_dir, root_manifest)) = &workspace_root {
            if !input_files.contains(&root_manifest.path) {
                input_files.push(root_manifest.path.clone());
            }

            for (name, source) in root_manifest
                .get_tool_uv()
                .iter()
                .flat_map(|uv| &uv.sources)
            {
                uv_sources.insert(normalize_distribution_name(name), (root_dir, source));
            }
        }

        if is_uv {
            for (name, source) in manifest.get_tool_uv().iter().flat_map(|uv| &uv.sources) {
                uv_sources.insert(normalize_distribution_name(name), (project_root, source));
            }
        }

        let mut add_dependency = |dep_id: &Id, scope: DependencyScope, label: &str| {
            // A dependency in multiple tables keeps the first, and most important, scope
            if dep_id != id
                && !project_output
                    .dependencies
                    .iter()
                    .any(|dep| &dep.id == dep_id)
            {
                project_output.dependencies.push(ProjectDependency {
                    id: dep_id.to_owned(),
                    scope,
                    via: Some(format!("requirement {label}")),
                });
            }
        };

        let resolve_path =
            |dir: &VirtualPath, path: &str| roots.get(&normalize_path(&dir.join(path)));

        let mut extract_implicit_deps = |reqs: Vec<Requirement>, scope: DependencyScope| {
            for req in reqs {
                let req_name = normalize_distribution_name(req.name.as_ref());

                // A URL is never local
                if matches!(req.version_or_url, Some(VersionOrUrl::Url(_))) {
                    continue;
                }

                let dep_id = match uv_sources.get(&req_name) {
                    // A uv source decides: a path links to the project at that path,
                    // and a workspace source to the member with the name
                    Some((dir, source)) => source
                        .get_paths()
                        .into_iter()
                        .find_map(|path| resolve_path(dir, path))
                        .or_else(|| {
                            if source.is_workspace() {
                                names.get(&req_name)
                            } else {
                                None
                            }
                        }),
                    // Otherwise only a requirement without a version is local
                    None if req.version_or_url.is_none() => names.get(&req_name),
                    None => None,
                };

                if let Some(dep_id) = dep_id {
                    add_dependency(dep_id, scope, req.name.as_ref());
                }
            }
        };

        extract_implicit_deps(manifest.get_dependencies(), DependencyScope::Production);
        extract_implicit_deps(
            manifest.get_optional_dependencies(),
            DependencyScope::Production,
        );
        extract_implicit_deps(
            manifest.get_dependency_groups(),
            DependencyScope::Development,
        );

        if is_uv {
            extract_implicit_deps(
                manifest.get_uv_dev_dependencies(),
                DependencyScope::Development,
            );
        }

        // Poetry only installs path dependencies locally, as versions come from the index
        if is_poetry && let Some(poetry) = manifest.get_tool_poetry() {
            let tables = [(&poetry.dependencies, DependencyScope::Production)]
                .into_iter()
                .chain(
                    poetry
                        .group
                        .values()
                        .map(|group| (&group.dependencies, DependencyScope::Development)),
                )
                .chain([(&poetry.dev_dependencies, DependencyScope::Development)]);

            for (deps, scope) in tables {
                for (dep_name, dep) in deps {
                    if let Some(dep_id) = dep
                        .get_paths()
                        .into_iter()
                        .find_map(|path| resolve_path(project_root, path))
                    {
                        add_dependency(dep_id, scope, dep_name);
                    }
                }
            }
        }

        output
            .extended_projects
            .insert(id.to_owned(), project_output);

        if !input_files.contains(&manifest.path) {
            input_files.push(manifest.path.clone());
        }
    }

    output.input_files = input_files;

    Ok(Json(output))
}

type UvWorkspaceRoot = (VirtualPath, Rc<PyProjectToml>);

/// Find the uv workspace that a project belongs to, by traversing upwards
/// to the first `pyproject.toml` with `[tool.uv.workspace]`, like uv.
fn find_uv_workspace_root(
    project_root: &VirtualPath,
    cache: &mut BTreeMap<PathBuf, Option<Rc<PyProjectToml>>>,
) -> AnyResult<Option<UvWorkspaceRoot>> {
    let mut found = None;

    locate_root_many_with_check(project_root, &["pyproject.toml"], |dir| {
        let key = dir.to_path_buf();

        if !cache.contains_key(&key) {
            let manifest = PyProjectToml::load(dir.join("pyproject.toml"))?;

            cache.insert(
                key.clone(),
                manifest.is_uv_workspace().then(|| Rc::new(manifest)),
            );
        }

        if let Some(Some(manifest)) = cache.get(&key) {
            found = Some((dir.to_owned(), Rc::clone(manifest)));

            return Ok(true);
        }

        Ok(false)
    })?;

    Ok(found)
}

/// Activate the closest venv, like its `activate` script does.
fn activate_venv(
    config: &PythonToolchainConfig,
    current_dir: &VirtualPath,
    output: &mut ExtendCommandOutput,
) -> AnyResult<()> {
    let Some(venv_parent) = locate_root(current_dir, &config.venv_name) else {
        return Ok(());
    };

    let Some(venv_root) = venv_parent.join(&config.venv_name).to_real_path()? else {
        return Ok(());
    };

    output.paths.push(venv_root.join("Scripts").to_path_buf());
    output.paths.push(venv_root.join("bin").to_path_buf());

    // Env values are passed on as they are, so they need the real path,
    // which Poetry and `uv pip` install into, unlike `.venv` alone
    let venv_root = venv_root.to_string_lossy().to_string();

    output.env.insert("VIRTUAL_ENV".into(), venv_root.clone());

    // uv ignores `VIRTUAL_ENV` for projects, and always uses `.venv`. Only set it for
    // other names, as an absolute path applies to every project that uv touches.
    if config.package_manager == Some(PythonPackageManager::Uv) && config.venv_name != ".venv" {
        output
            .env
            .insert("UV_PROJECT_ENVIRONMENT".into(), venv_root);
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_command(
    Json(input): Json<ExtendCommandInput>,
) -> FnResult<Json<ExtendCommandOutput>> {
    let config = parse_toolchain_config_schema::<PythonToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendTaskCommandOutput::default();

    activate_venv(&config, &input.current_dir, &mut output)?;

    // uv picks its Python from `.python-version` and `requires-python` before PATH,
    // so point its commands and tasks at the configured version (D2)
    if matches!(
        config.package_manager,
        Some(PythonPackageManager::Uv | PythonPackageManager::UvPip)
    ) && let Some(request) = config.version.as_ref().and_then(to_uv_python_request)
    {
        output.env.insert("UV_PYTHON".into(), request);
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
        PythonPackageManager::Poetry => vec!["pyproject.toml"],
        PythonPackageManager::Uv => vec!["pyproject.toml"],
    };

    let lock_names = match package_manager {
        PythonPackageManager::Pip | PythonPackageManager::UvPip => {
            vec!["pylock.toml", "requirements.txt"]
        }
        PythonPackageManager::Poetry => vec!["poetry.lock"],
        PythonPackageManager::Uv => vec!["uv.lock"],
    };

    // First attempt: find lock files
    if let Some(root) = locate_root_many(&input.starting_dir, &lock_names) {
        output.members =
            PyProjectToml::load(root.join("pyproject.toml"))?.extract_members(package_manager);
        output.root = Some(root);
    }

    // Second attempt: find workspace-compatible manifest files
    if output.root.is_none() {
        locate_root_many_with_check(&input.starting_dir, &manifest_names, |root| {
            let manifest = PyProjectToml::load(root.join("pyproject.toml"))?;
            let mut found = false;

            if let Some(members) = manifest.extract_members(package_manager) {
                output.root = Some(root.to_owned());
                output.members = Some(members);
                found = true;
            }

            Ok(found)
        })?;
    }

    // Last attempt: find a manifest file (project only)
    if output.root.is_none() {
        output.root = locate_root_many(&input.starting_dir, &manifest_names);
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

    // `uv pip` installs with the pip toolchain's args (D1)
    let package_manager_id = match package_manager {
        PythonPackageManager::Pip | PythonPackageManager::UvPip => "pip",
        PythonPackageManager::Poetry => "poetry",
        PythonPackageManager::Uv => "uv",
    };

    let package_manager_config: SharedPackageManagerConfig = match &input.project {
        Some(project) => load_project_toolchain_config(&project.id, package_manager_id)?,
        None => load_toolchain_config(package_manager_id)?,
    };

    let mut command = match package_manager {
        PythonPackageManager::Pip | PythonPackageManager::UvPip => {
            let mut cmd = if package_manager == PythonPackageManager::Pip {
                ExecCommandInput::new("python", ["-m", "pip", "install"])
            } else {
                ExecCommandInput::new("uv", ["pip", "install"])
            };

            if !append_pip_install_sources(&input.root, package_manager, &mut cmd.args)? {
                return Ok(Json(output));
            }

            cmd
        }
        PythonPackageManager::Poetry => {
            // Unlike `install`, `sync` removes the packages it excludes
            if input.production {
                if is_poetry_v1(package_manager_config.version.as_ref()) {
                    ExecCommandInput::new("poetry", ["install", "--sync", "--only", "main"])
                } else {
                    ExecCommandInput::new("poetry", ["sync", "--only", "main"])
                }
            } else {
                ExecCommandInput::new("poetry", ["install"])
            }
        }
        PythonPackageManager::Uv => {
            let mut cmd = ExecCommandInput::new("uv", ["sync"]);

            if input.packages.is_empty() {
                // A plain `uv sync` only syncs the root package, and uninstalls
                // the other members, when the workspace root is a package itself
                if !has_arg(&package_manager_config.install_args, "--all-packages")
                    && !has_arg(&package_manager_config.install_args, "--package")
                    && PyProjectToml::load(input.root.join("pyproject.toml"))?.is_uv_workspace()
                {
                    cmd.args.push("--all-packages".into());
                }
            } else {
                for package_name in input.packages {
                    cmd.args.push("--package".into());
                    cmd.args.push(package_name);
                }
            }

            // Every default group, not only `dev`
            if input.production {
                cmd.args.push("--no-default-groups".into());
            }

            // Fail on a stale lockfile in CI, instead of silently re-locking it (D3)
            if get_host_environment()?.ci
                && input.root.join("uv.lock").exists()
                && !has_uv_lock_mode(&package_manager_config.install_args)?
            {
                cmd.args.push("--locked".into());
            }

            cmd
        }
    };

    command.args.extend(package_manager_config.install_args);

    if matches!(
        package_manager,
        PythonPackageManager::Uv | PythonPackageManager::UvPip
    ) {
        append_uv_fallback_args(&config, &mut command.args);
    }

    command.cwd = Some(input.root.clone());

    output.install_command = Some(command.into());

    Ok(Json(output))
}

/// Append what pip and `uv pip` install from: a requirements file, or else the
/// project in `pyproject.toml`. Returns false when there's nothing to install.
fn append_pip_install_sources(
    root: &VirtualPath,
    package_manager: PythonPackageManager,
    args: &mut Vec<String>,
) -> AnyResult<bool> {
    if root.join("requirements.txt").exists() {
        args.push("-r".into());
        args.push("requirements.txt".into());
    } else if root.join("requirements.in").exists() {
        args.push("-r".into());
        args.push("requirements.in".into());
    } else {
        let manifest = PyProjectToml::load(root.join("pyproject.toml"))?;

        if manifest.project.is_none() {
            return Ok(false);
        }

        // Like uv, only install the project itself when it's built by a backend,
        // as setuptools' discovery fails for flat layouts without one
        if manifest.has_build_system() || manifest.has_dynamic_dependencies() {
            args.push("-e".into());
            args.push(".".into());
        } else {
            let deps = manifest.get_dependencies();

            if deps.is_empty() {
                return Ok(false);
            }

            // pip can't install only the dependencies of a project
            if package_manager == PythonPackageManager::UvPip {
                args.push("-r".into());
                args.push("pyproject.toml".into());
            } else {
                args.extend(deps.into_iter().map(|req| req.to_string()));
            }
        }
    }

    if root.join("constraints.txt").exists() {
        args.push("-c".into());
        args.push("constraints.txt".into());
    }

    Ok(true)
}

/// Whether uv already checks or skips the lockfile, as it rejects `--locked`
/// twice, or with `--frozen`, including from `UV_LOCKED` and `UV_FROZEN`.
fn has_uv_lock_mode(args: &[String]) -> AnyResult<bool> {
    if has_arg(args, "--locked") || has_arg(args, "--frozen") {
        return Ok(true);
    }

    // ADR-0004
    for name in ["UV_LOCKED", "UV_FROZEN"] {
        if get_host_env_var(name)?.is_some_and(|value| !value.is_empty()) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Poetry 1.x has no `sync` command. Without a version, assume Poetry 2.
fn is_poetry_v1(version: Option<&UnresolvedVersionSpec>) -> bool {
    match version {
        Some(UnresolvedVersionSpec::Version(version)) => version.major == 1,
        Some(UnresolvedVersionSpec::Requirement(req)) => {
            // ADR-0005.1
            req.major == Some(1) && !req.matches(&Version::parse("2.0.0").unwrap())
        }
        _ => false,
    }
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();

    let result = match input.path.file_name().and_then(|name| name.to_str()) {
        Some("pylock.toml") => parse_pylock_toml(&input.path, &mut output),
        Some("poetry.lock") => parse_poetry_lock(&input.path, &mut output),
        Some("requirements.txt") => parse_requirements_txt(&input.path, &mut output),
        Some("uv.lock") => parse_uv_lock(&input.path, &mut output),
        _ => Ok(()),
    };

    // ADR-0008.8
    if let Err(error) = result {
        return Err(anyhow!("Failed to parse <file>{}</file>: {error}", input.path).into());
    }

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
        PythonPackageManager::Poetry => (
            ExecCommandInput::new("python", ["-m", "venv", &config.venv_name]),
            "poetry",
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

    command.args.extend(package_manager_config.venv_args);

    if matches!(
        package_manager,
        PythonPackageManager::Uv | PythonPackageManager::UvPip
    ) {
        append_uv_fallback_args(&config, &mut command.args);
    }

    let venv_dir = input.root.join(&config.venv_name);

    if venv_dir.exists() {
        // When the configured Python changed, recreate the venv with it. Deleting it
        // also takes the old venv's Python off PATH, which would otherwise run the
        // create command, and recreate the venv on the old version.
        if is_venv_stale(&venv_dir, config.version.as_ref()) {
            fs::remove_dir_all(&venv_dir)?;
        }
        // If `--clear` is not passed (the same for all managers), then we
        // can just avoid the overhead of initializing the venv again
        else if !has_arg(&command.args, "--clear") {
            return Ok(Json(output));
        }
    }

    command.cwd = Some(input.root.clone());

    output.commands.push(command.into());

    Ok(Json(output))
}

/// Whether the venv was created with a Python that doesn't match `version`.
fn is_venv_stale(venv_dir: &VirtualPath, version: Option<&UnresolvedVersionSpec>) -> bool {
    let Some(version) = version else {
        return false;
    };

    let cfg_path = venv_dir.join("pyvenv.cfg");

    let Some(venv_version) = fs::read_file(&cfg_path)
        .ok()
        .and_then(|content| parse_pyvenv_cfg_version(&content))
    else {
        return false;
    };

    if is_venv_version_compatible(&venv_version, version) {
        return false;
    }

    debug!("Recreating {venv_dir}, as its Python {venv_version} doesn't match {version}");

    true
}

/// Whether the args include a flag, as `--flag` or `--flag=value`.
fn has_arg(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| {
        arg == name
            || arg
                .strip_prefix(name)
                .is_some_and(|rest| rest.starts_with('='))
    })
}

/// Append the args that keep uv on moon's Python, after the configured ones.
/// uv rejects some flags when they're repeated, or combined with others.
fn append_uv_fallback_args(config: &PythonToolchainConfig, args: &mut Vec<String>) {
    let mut fallback_args = vec![];

    // The user opted into uv's own Python
    let uses_uv_python = has_arg(args, "--managed-python") || has_arg(args, "--python-preference");

    if config.version.is_some() && !uses_uv_python {
        for arg in ["--no-managed-python", "--no-python-downloads"] {
            if !has_arg(args, arg) {
                fallback_args.push(arg.to_owned());
            }
        }
    }

    if !has_arg(args, "--no-progress") {
        fallback_args.push("--no-progress".into());
    }

    args.extend(fallback_args);
}
