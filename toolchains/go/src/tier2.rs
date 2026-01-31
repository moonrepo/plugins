use crate::config::GoToolchainConfig;
use crate::go_mod::parse_go_mod;
use crate::go_sum::GoSum;
use crate::go_work::GoWork;
use extism_pdk::*;
use moon_config::{BinEntry, DependencyScope};
use moon_pdk::{
    get_host_env_var, get_host_environment, locate_root, parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let mut output = ExtendProjectGraphOutput::default();

    // First pass, gather all packages and their manifests
    let mut packages = BTreeMap::default();

    for (id, source) in input.project_sources {
        let go_mod_path = input.context.workspace_root.join(source).join("go.mod");

        if go_mod_path.exists() {
            let manifest = parse_go_mod(fs::read_file(&go_mod_path)?)?;

            packages.insert(manifest.module.clone(), (id, manifest));

            if let Some(file) = go_mod_path.virtual_path() {
                output.input_files.push(file);
            }
        }
    }

    // Second pass, extract packages and their relationships
    for (id, manifest) in packages.values() {
        let mut project_output = ExtendProjectOutput {
            alias: Some(manifest.module.clone()),
            ..Default::default()
        };

        for dep in &manifest.require {
            if !dep.indirect && packages.contains_key(&dep.module.module_path) {
                project_output.dependencies.push(ProjectDependency {
                    id: Id::raw(dep.module.module_path.clone()),
                    scope: DependencyScope::Production,
                    via: Some(format!("module {}", dep.module.module_path)),
                });
            }
        }

        output
            .extended_projects
            .insert(id.to_owned(), project_output);
    }

    Ok(Json(output))
}

fn gather_shared_paths(
    env: &HostEnvironment,
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(globals_dir) = globals_dir
        && globals_dir.real_path().is_some()
    {
        // Avoid the host env overhead if we already
        // have a valid globals directory!
        return Ok(());
    }

    if let Some(dir) = var::get::<String>("bin_dir")? {
        paths.push(PathBuf::from(dir));
    } else {
        let maybe_dir = if let Some(value) = get_host_env_var("GOBIN")? {
            Some(PathBuf::from(value))
        } else if let Some(value) = get_host_env_var("GOPATH")? {
            Some(PathBuf::from(value).join("bin"))
        } else {
            env.home_dir.join("go").join("bin").real_path()
        };

        if let Some(dir) = maybe_dir {
            if let Some(dir_str) = dir.to_str() {
                var::set("bin_dir", dir_str)?;
            }

            paths.push(dir);
        }
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let env = get_host_environment()?;

    // Always include Go specific paths for all commands
    gather_shared_paths(&env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();
    let env = get_host_environment()?;

    // Always include Go specific paths for all commands
    gather_shared_paths(&env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let mut output = LocateDependenciesRootOutput::default();

    // Find `go.work` first
    if config.workspaces
        && let Some(root) = locate_root(&input.starting_dir, "go.work")
    {
        let go_work = GoWork::parse(fs::read_file(root.join("go.work"))?)?;

        if !go_work.modules.is_empty() {
            output.members = Some(go_work.modules);
        }

        output.root = root.virtual_path();
    }

    // Then `go.sum` second
    if output.root.is_none()
        && let Some(root) = locate_root(&input.starting_dir, "go.sum")
    {
        output.root = root.virtual_path();
    }

    // Otherwise assume `go.mod`
    if output.root.is_none()
        && let Some(root) = locate_root(&input.starting_dir, "go.mod")
    {
        output.root = root.virtual_path();
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let mut output = InstallDependenciesOutput::default();

    if config.workspaces && input.root.join("go.work").exists() {
        output.install_command = Some(
            ExecCommandInput::new("go", ["work", "sync"])
                .cwd(input.root.clone())
                .into(),
        );
    }

    if output.install_command.is_none() && input.root.join("go.mod").exists() {
        output.install_command = Some(
            ExecCommandInput::new("go", ["mod", "download"])
                .cwd(input.root.clone())
                .into(),
        );

        if config.tidy_on_change && input.root.join("go.sum").exists() {
            output.dedupe_command = Some(
                ExecCommandInput::new("go", ["mod", "tidy"])
                    .cwd(input.root)
                    .into(),
            );
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();
    let go_sum = GoSum::parse(fs::read_file(input.path)?)?;

    for (module, entry) in go_sum.dependencies {
        output
            .dependencies
            .entry(module)
            .or_default()
            .push(LockDependency {
                hash: entry
                    .checksum
                    .strip_prefix("h1:") // sha256
                    .map(|hash| hash.to_owned()),
                version: Some(entry.version),
                ..Default::default()
            });
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();

    match input.path.file_name().and_then(|name| name.to_str()) {
        Some("go.mod") => {
            let go_mod = parse_go_mod(fs::read_file(input.path)?)?;

            for dep in go_mod.require {
                // Ignore transitive deps, as we only care about
                // direct project deps during task hashing
                if dep.indirect {
                    continue;
                }

                output.dependencies.insert(
                    dep.module.module_path,
                    ManifestDependency::Version(UnresolvedVersionSpec::parse(dep.module.version)?),
                );
            }
        }
        Some("go.work") => {
            // Do nothing for now...
        }
        _ => {}
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let mut output = SetupEnvironmentOutput::default();

    // Install binaries
    // https://go.dev/ref/mod#go-install
    // https://pkg.go.dev/cmd/go#hdr-Compile_and_install_packages_and_dependencies
    if !config.bins.is_empty() {
        let env = get_host_environment()?;
        let mut bins_by_version = BTreeMap::default();

        for bin in &config.bins {
            let name = match bin {
                BinEntry::Name(inner) => inner,
                BinEntry::Config(cfg) => {
                    if cfg.local && env.ci {
                        continue;
                    } else {
                        cfg.bin.as_str()
                    }
                }
            };

            let (module, version) = name.split_once('@').unwrap_or((name, "latest"));
            let base_module = get_base_module(module);

            bins_by_version
                .entry(format!("{base_module}@{version}"))
                .or_insert_with(Vec::new)
                .push(name);
        }

        for (version, bins) in bins_by_version {
            let mut args = vec!["install", "-v"];
            args.extend(bins);

            output.commands.push(
                ExecCommand::new(ExecCommandInput::new("go", args).cwd(input.root.to_owned()))
                    .cache(format!("go-bins-{version}")),
            );
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn hash_task_contents(
    Json(_): Json<HashTaskContentsInput>,
) -> FnResult<Json<HashTaskContentsOutput>> {
    let env = get_host_environment()?;

    let mut map = json::Map::default();
    map.insert("os".into(), json::Value::String(env.os.to_string()));
    map.insert("arch".into(), json::Value::String(env.arch.to_string()));
    map.insert("libc".into(), json::Value::String(env.libc.to_string()));

    Ok(Json(HashTaskContentsOutput {
        contents: vec![json::Value::Object(map)],
    }))
}

fn get_base_module(module: &str) -> String {
    let mut parts = module.split('/');
    let mut base = String::new();

    // github.com
    base.push_str(parts.next().unwrap_or_default());
    base.push('/');

    // moonrepo
    base.push_str(parts.next().unwrap_or_default());
    base.push('/');

    // plugins
    base.push_str(parts.next().unwrap_or_default());

    base
}
