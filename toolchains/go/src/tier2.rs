use crate::config::GoToolchainConfig;
use crate::go_mod::{Replacement, parse_go_mod};
use crate::go_sum::GoSum;
use crate::go_work::GoWork;
use crate::package_graph::{GoPackageGraph, is_version_segment, module_dir_candidates};
use extism_pdk::*;
use moon_config::BinEntry;
use moon_pdk::{
    HostLogInput, HostLogTarget, VirtualPathExt, command_exists, get_host_env_var,
    get_host_environment, host_log, locate_root, parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use starbase_utils::fs;
use std::path::PathBuf;

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let env = get_host_environment()?;

    let mut graph = GoPackageGraph::new(
        input.context.workspace_root,
        config,
        command_exists(env, "go"),
    );

    // First pass through, we figure out what projects we have and what their root import path is
    graph.load_projects(input.project_sources)?;

    let mut output = ExtendProjectGraphOutput::default();

    // On the second pass, we work through all the projects and resolve their dependencies
    for project in graph.projects() {
        let project_output = ExtendProjectOutput {
            alias: project.alias.clone(),
            dependencies: graph.project_dependencies(project)?,
            ..Default::default()
        };

        if project_output.alias.is_some() || !project_output.dependencies.is_empty() {
            output
                .extended_projects
                .insert(project.id.to_owned(), project_output);
        }
    }

    output.input_files = graph.into_input_files();

    Ok(Json(output))
}

fn gather_shared_paths(
    env: &HostEnvironment,
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    // moon only adds the globals directory to `PATH` when a version
    // is configured, so binaries from `bins` would be missing otherwise
    if let Some(globals_dir) = globals_dir
        && let Some(dir) = globals_dir.to_real_path()?
    {
        paths.push(dir.to_path_buf());

        return Ok(());
    }

    if let Some(dir) = var::get::<String>("bin_dir")? {
        paths.push(PathBuf::from(dir));
    } else {
        let maybe_dir = if let Some(value) = get_host_env_var("GOBIN")? {
            Some(PathBuf::from(value))
        } else if let Some(value) = get_host_env_var("GOPATH")? {
            // A list of paths, where `go install` uses the first
            value
                .split(if env.os.is_windows() { ';' } else { ':' })
                .next()
                .map(|path| PathBuf::from(path).join("bin"))
        } else {
            env.home_dir
                .join("go")
                .join("bin")
                .to_real_path()?
                .map(|path| path.to_path_buf())
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
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendTaskCommandOutput::default();
    let env = get_host_environment()?;

    if let Some(mode) = get_toolchain_mode(&config)? {
        output.env.insert("GOTOOLCHAIN".into(), mode);
    }

    // Always include Go specific paths for all commands
    gather_shared_paths(env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let mut output = ExtendTaskScriptOutput::default();
    let env = get_host_environment()?;

    if let Some(mode) = get_toolchain_mode(&config)? {
        output.env.insert("GOTOOLCHAIN".into(), mode);
    }

    // Always include Go specific paths for all commands
    gather_shared_paths(env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let mut output = LocateDependenciesRootOutput::default();

    // Find `go.work` first
    if use_workspaces(&config)?
        && let Some(root) = locate_root(&input.starting_dir, "go.work")
    {
        let go_work = GoWork::parse(fs::read_file(root.join("go.work"))?)?;

        if !go_work.modules.is_empty() {
            output.members = Some(go_work.modules);
        }

        output.root = Some(root);
    }

    // Otherwise the nearest `go.mod`, as `go.sum` always sits beside it.
    // Every directory below it is part of the module (nested modules find
    // their own `go.mod` first), so projects there share its install
    if output.root.is_none()
        && let Some(root) = locate_root(&input.starting_dir, "go.mod")
    {
        output.members = Some(vec!["**/*".into()]);
        output.root = Some(root);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let mut output = InstallDependenciesOutput::default();

    // Go 1.14+ builds from a committed `vendor` directory by default,
    // so there's nothing to download
    if input.root.join("vendor").join("modules.txt").exists() {
        return Ok(Json(output));
    }

    // In workspace mode, this downloads the workspace's build list.
    // Unlike `go work sync`, it doesn't rewrite each member's `go.mod`
    let in_workspace = use_workspaces(&config)? && input.root.join("go.work").exists();

    let mode = get_toolchain_mode(&config)?;

    let create_command = |args: [&str; 2]| {
        let mut command = ExecCommandInput::new("go", args).cwd(input.root.clone());

        if let Some(mode) = &mode {
            command.env.insert("GOTOOLCHAIN".into(), mode.to_owned());
        }

        ExecCommand::new(command)
    };

    if in_workspace || input.root.join("go.mod").exists() {
        output.install_command = Some(create_command(["mod", "download"]));
    }

    if !in_workspace && config.tidy_on_change && input.root.join("go.sum").exists() {
        output.dedupe_command = Some(create_command(["mod", "tidy"]));
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();
    let go_sum = GoSum::parse(fs::read_file(input.path)?)?;

    for (module, entries) in go_sum.dependencies {
        output.dependencies.insert(
            module,
            entries
                .into_iter()
                .map(|entry| LockDependency {
                    hash: entry
                        .checksum
                        .strip_prefix("h1:") // sha256
                        .map(|hash| hash.to_owned()),
                    version: Some(entry.version),
                    ..Default::default()
                })
                .collect(),
        );
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
            // moon reads every `go.mod` below a root, which includes
            // intentionally invalid ones in `testdata`
            let go_mod = match parse_go_mod(fs::read_file(&input.path)?) {
                Ok(go_mod) => go_mod,
                Err(error) => {
                    host_log!(
                        warn,
                        "Failed to parse <file>{}</file>, skipping: {error}",
                        input.path
                    );

                    return Ok(Json(output));
                }
            };

            // Indirect requires are part of the build too (Go 1.17+),
            // so a bump to one must change task hashes
            for dep in go_mod.require {
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

fn is_bin_installed(
    env: &HostEnvironment,
    globals_dir: Option<&VirtualPath>,
    module: &str,
    version: &str,
) -> bool {
    // Without a registry to inspect (like Cargo's `.crates.toml`), we can't
    // verify which version an installed binary is, so entries pinned to a
    // version, branch, or commit are always included. Their command only
    // executes when the environment fingerprint changes, like a version bump
    if version != "latest" {
        return false;
    }

    let Some(globals_dir) = globals_dir else {
        return false;
    };

    globals_dir
        .join(env.os.get_exe_name(get_bin_name(module)))
        .exists()
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

        let mode = get_toolchain_mode(&config)?;

        // Install where we check for binaries, and where tasks find them
        let gobin = match &input.globals_dir {
            Some(dir) => dir.to_real_path()?.map(|dir| dir.to_string()),
            None => None,
        };

        for bin in &config.bins {
            let (name, force, bin_args) = match bin {
                BinEntry::String(inner) => (inner.as_str(), false, [].as_slice()),
                BinEntry::Object(cfg) => {
                    if cfg.local && env.ci {
                        continue;
                    }

                    (cfg.bin.as_str(), cfg.force, cfg.args.as_slice())
                }
            };

            let (module, version) = name.split_once('@').unwrap_or((name, "latest"));

            if !force && is_bin_installed(env, input.globals_dir.as_ref(), module, version) {
                continue;
            }

            // One command per bin, as Go requires every package in a
            // `go install` to come from the same module and version
            let mut command = ExecCommandInput::new("go", ["install", "-v"]);
            command.args.extend(bin_args.iter().cloned());
            command.args.push(name.to_owned());
            command.cwd = Some(input.root.clone());

            if let Some(gobin) = &gobin {
                command.env.insert("GOBIN".into(), gobin.to_owned());
            }

            if let Some(mode) = &mode {
                command.env.insert("GOTOOLCHAIN".into(), mode.to_owned());
            }

            output.commands.push(
                ExecCommand::new(command)
                    .cache(CacheStrategy::Memory)
                    .label(format!("go-bin-{name}")),
            );
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn hash_task_contents(
    Json(input): Json<HashTaskContentsInput>,
) -> FnResult<Json<HashTaskContentsOutput>> {
    let env = get_host_environment()?;

    let mut map = json::Map::default();
    map.insert("os".into(), json::Value::String(env.os.to_string()));
    map.insert("arch".into(), json::Value::String(env.arch.to_string()));
    map.insert("libc".into(), json::Value::String(env.libc.to_string()));

    let mut contents = vec![json::Value::Object(map)];

    if let Some(module) = hash_owning_module(&input.context.workspace_root, &input.project.source)?
    {
        contents.push(module);
    }

    Ok(Json(HashTaskContentsOutput { contents }))
}

// moon only hashes the dependencies of a `go.mod` in the project's own
// directory, so for a project without one, like in a single-module
// monorepo, hash those of the module it belongs to
fn hash_owning_module(
    workspace_root: &VirtualPath,
    source: &str,
) -> AnyResult<Option<json::Value>> {
    let source = if source == "." { "" } else { source };

    for dir in module_dir_candidates(source) {
        let go_mod_path = if dir.is_empty() {
            workspace_root.join("go.mod")
        } else {
            workspace_root.join(dir).join("go.mod")
        };

        if !go_mod_path.exists() {
            continue;
        }

        if dir == source {
            return Ok(None);
        }

        let go_mod = parse_go_mod(fs::read_file(go_mod_path)?)?;

        let require = go_mod
            .require
            .into_iter()
            .map(|dep| (dep.module.module_path, dep.module.version))
            .collect::<std::collections::BTreeMap<_, _>>();

        let replace = go_mod
            .replace
            .into_iter()
            .map(|item| {
                let target = match item.replacement {
                    Replacement::FilePath(path) => path,
                    Replacement::Module(module) => {
                        format!("{} {}", module.module_path, module.version)
                    }
                };

                match item.version {
                    Some(version) => format!("{} {version} => {target}", item.module_path),
                    None => format!("{} => {target}", item.module_path),
                }
            })
            .collect::<Vec<_>>();

        return Ok(Some(json::json!({
            "go": go_mod.go,
            "toolchain": go_mod.toolchain,
            "require": require,
            "replace": replace,
        })));
    }

    Ok(None)
}

// Since Go 1.21, `go` downloads and runs a newer toolchain when `go.mod`
// requires one, so pin it to the configured version, unless the host
// chose a mode itself. A `go.mod` that needs a newer Go then fails
fn get_toolchain_mode(config: &GoToolchainConfig) -> AnyResult<Option<String>> {
    if config.version.is_none()
        || get_host_env_var("GOTOOLCHAIN")?.is_some_and(|value| !value.is_empty())
    {
        return Ok(None);
    }

    Ok(Some("local".into()))
}

// `GOWORK=off` disables workspace mode, like `workspaces: false`
fn use_workspaces(config: &GoToolchainConfig) -> AnyResult<bool> {
    Ok(config.workspaces && get_host_env_var("GOWORK")?.is_none_or(|value| value != "off"))
}

// The executable is named after the last segment of the module path,
// excluding a major version suffix: `github.com/foo/bar/v2` -> `bar`
fn get_bin_name(module: &str) -> &str {
    let mut segments = module.rsplit('/');
    let last = segments.next().unwrap_or(module);

    if is_version_segment(last) {
        return segments.next().unwrap_or(last);
    }

    last
}
