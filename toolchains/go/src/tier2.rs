use crate::config::GoToolchainConfig;
use crate::go_mod::{ModuleReplacement, Replacement, parse_go_mod};
use crate::go_sum::GoSum;
use crate::go_work::GoWork;
use crate::package_graph::{
    ModuleResolver, execute_go_list, import_within, is_version_segment, resolve_source_path,
};
use extism_pdk::*;
use moon_config::{BinEntry, DependencyScope};
use moon_pdk::{
    VirtualPathExt, command_exists, get_host_env_var, get_host_environment, locate_root,
    parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use starbase_utils::fs;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

struct GoProject {
    id: Id,
    root: VirtualPath,
    /// Module path as declared when the project owns its `go.mod`
    alias: Option<String>,
    import_path: Option<String>,
    /// Direct module requires from the project's own `go.mod`
    requires: Vec<GoRequire>,
}

struct GoRequire {
    module_path: String,
    target: GoRequireTarget,
}

// What a `require` actually resolves to once `replace` directives are
// applied, which determines whether it can link to a local project.
enum GoRequireTarget {
    /// Required by version; only links when the environment wires the
    /// module to local source (a `go.work` workspace)
    Module,
    /// Replaced by a local directory (workspace-relative), so it always
    /// consumes local source
    LocalSource(String),
    /// Replaced by another module, or a path outside the workspace, so it
    /// can never be a local project
    External,
}

// All the state relationship inference works from: the resolved projects,
// the import-path prefix map they resolve against, and the module resolver
// used to build both.
struct GoProjectGraph {
    workspace_root: VirtualPath,
    config: GoToolchainConfig,
    go_exists: bool,
    resolver: ModuleResolver,
    projects: Vec<GoProject>,
    /// Project lookup by workspace-relative source dir, for `replace`
    /// directives that point at local directories
    source_to_id: BTreeMap<String, Id>,
    /// Import path each project resolves to, matched by prefix
    package_prefixes: Vec<(String, Id)>,
    /// The `go.mod` files backing the resolved import paths
    input_files: Vec<VirtualPath>,
}

impl GoProjectGraph {
    // First pass: resolve every project to its import path and manifest, and
    // build the prefix map that imports are resolved against.
    fn load_projects(&mut self, sources: BTreeMap<Id, String>) -> AnyResult<()> {
        for (id, source) in sources {
            let root = self.workspace_root.join(&source);
            let source = if source == "." { "" } else { source.as_str() };

            self.source_to_id.insert(source.to_owned(), id.clone());

            let mut project = GoProject {
                id,
                root,
                alias: None,
                import_path: None,
                requires: vec![],
            };

            if let Some((mod_dir, import_path)) = self.resolver.import_path(source)? {
                let go_mod_path = self.resolver.go_mod_path(&mod_dir);

                if !self.input_files.contains(&go_mod_path) {
                    self.input_files.push(go_mod_path);
                }

                // An ancestor's requires describe the whole module, so they
                // only participate for the project that owns the `go.mod`
                if mod_dir == source
                    && let Some(manifest) = self.resolver.manifest(&mod_dir)
                {
                    if manifest.module != project.id.as_str() {
                        project.alias = Some(manifest.module.clone());
                    }

                    project.requires = manifest
                        .require
                        .iter()
                        .filter(|dep| !dep.indirect)
                        .map(|dep| {
                            let module_path = dep.module.module_path.clone();

                            // A `replace` directive changes what the require
                            // resolves to, so it takes precedence over
                            // matching import paths
                            let target = match manifest
                                .replace
                                .iter()
                                .find(|replacement| replacement.module_path == module_path)
                            {
                                Some(ModuleReplacement {
                                    replacement: Replacement::FilePath(path),
                                    ..
                                }) => resolve_source_path(source, path).map_or(
                                    GoRequireTarget::External,
                                    GoRequireTarget::LocalSource,
                                ),
                                Some(_) => GoRequireTarget::External,
                                None => GoRequireTarget::Module,
                            };

                            GoRequire {
                                module_path,
                                target,
                            }
                        })
                        .collect();
                }

                project.import_path = Some(import_path);
            }

            self.projects.push(project);
        }

        self.package_prefixes = self
            .projects
            .iter()
            .filter_map(|project| {
                project
                    .import_path
                    .clone()
                    .map(|path| (path, project.id.clone()))
            })
            .collect();

        Ok(())
    }

    // Resolves an import path to the project whose import path prefixes it.
    // The longest match wins, so the module root can't shadow projects
    // nested beneath it.
    fn resolve_import(&self, import_path: &str) -> Option<&Id> {
        self.package_prefixes
            .iter()
            .filter(|(prefix, _)| import_within(import_path, prefix))
            .max_by_key(|(prefix, _)| prefix.len())
            .map(|(_, id)| id)
    }

    // Second pass: infer one project's dependencies, picking the mechanism
    // the environment supports.
    fn project_dependencies(&self, project: &GoProject) -> AnyResult<Vec<ProjectDependency>> {
        let mut dependencies = vec![];

        // A project without an import path may still sit under a `go.work`,
        // where `go list` resolves imports in workspace mode
        if self.go_exists
            && (project.import_path.is_some() || project.root.join("go.work").exists())
        {
            dependencies = self.dependencies_from_go_list(project)?;
        }

        // Monorepos using go.work can be resolved purely off the modfile: the
        // workspace wires required sibling modules to their local source, so
        // a project's own requires reflect real local relationships. That
        // only matters when `go` isn't around to resolve them properly, while
        // requires replaced by a local directory always consume local source.
        let include_unreplaced = !self.go_exists
            && self.workspace_root.join("go.work").exists()
            && project.root.join("go.mod").exists();

        for dependency in self.dependencies_from_modfile(project, include_unreplaced) {
            if !dependencies.iter().any(|dep| dep.id == dependency.id) {
                dependencies.push(dependency);
            }
        }

        Ok(dependencies)
    }

    fn dependencies_from_modfile(
        &self,
        project: &GoProject,
        include_unreplaced: bool,
    ) -> Vec<ProjectDependency> {
        let mut dependencies = vec![];
        let mut seen = BTreeSet::new();

        for require in &project.requires {
            let dep_id = match &require.target {
                GoRequireTarget::LocalSource(path) => self.source_to_id.get(path),
                GoRequireTarget::External => None,
                GoRequireTarget::Module => include_unreplaced
                    .then(|| self.resolve_import(&require.module_path))
                    .flatten(),
            };

            if let Some(dep_id) = dep_id
                && dep_id != &project.id
                && seen.insert(dep_id)
            {
                dependencies.push(ProjectDependency {
                    id: dep_id.to_owned(),
                    scope: DependencyScope::Production,
                    via: Some(format!("module {}", require.module_path)),
                });
            }
        }

        dependencies
    }

    fn dependencies_from_go_list(&self, project: &GoProject) -> AnyResult<Vec<ProjectDependency>> {
        let mut dependencies = vec![];
        let mut seen = BTreeSet::new();

        for (enabled, test, scope) in [
            (
                self.config.infer_relationships,
                false,
                DependencyScope::Production,
            ),
            (
                self.config.infer_relationships_from_tests,
                true,
                DependencyScope::Development,
            ),
        ] {
            if !enabled {
                continue;
            }

            let imports = execute_go_list(
                &project.root,
                &self.config.infer_relationships_packages,
                test,
            )?;

            for import_path in imports {
                // `./...` also enumerates packages belonging to projects
                // nested inside this one; anything within the project's own
                // import path is ownership, not an import
                if let Some(own) = project.import_path.as_deref()
                    && import_within(&import_path, own)
                {
                    continue;
                }

                if let Some(dep_id) = self.resolve_import(&import_path)
                    && dep_id != &project.id
                    && seen.insert(dep_id)
                {
                    dependencies.push(ProjectDependency {
                        id: dep_id.to_owned(),
                        scope,
                        via: Some(format!("package {import_path}")),
                    });
                }
            }
        }

        Ok(dependencies)
    }
}

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let config = parse_toolchain_config_schema::<GoToolchainConfig>(input.toolchain_config)?;
    let env = get_host_environment()?;

    let mut graph = GoProjectGraph {
        resolver: ModuleResolver::new(input.context.workspace_root.clone()),
        workspace_root: input.context.workspace_root,
        config,
        go_exists: command_exists(env, "go"),
        projects: vec![],
        source_to_id: BTreeMap::default(),
        package_prefixes: vec![],
        input_files: vec![],
    };

    // First pass through, we figure out what projects we have and what their root import path is
    graph.load_projects(input.project_sources)?;

    let mut output = ExtendProjectGraphOutput::default();

    // On the second pass, we work through all the projects and resolve their dependencies
    for project in &graph.projects {
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

    output.input_files = graph.input_files;

    Ok(Json(output))
}

fn gather_shared_paths(
    env: &HostEnvironment,
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(globals_dir) = globals_dir
        && globals_dir.to_real_path()?.is_some()
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
    let mut output = ExtendTaskCommandOutput::default();
    let env = get_host_environment()?;

    // Always include Go specific paths for all commands
    gather_shared_paths(env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();
    let env = get_host_environment()?;

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
    if config.workspaces
        && let Some(root) = locate_root(&input.starting_dir, "go.work")
    {
        let go_work = GoWork::parse(fs::read_file(root.join("go.work"))?)?;

        if !go_work.modules.is_empty() {
            output.members = Some(go_work.modules);
        }

        output.root = Some(root);
    }

    // Then `go.sum` second
    if output.root.is_none() {
        output.root = locate_root(&input.starting_dir, "go.sum");
    }

    // Otherwise assume `go.mod`
    if output.root.is_none() {
        output.root = locate_root(&input.starting_dir, "go.mod");
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
        let mut bins_by_version = BTreeMap::default();

        for bin in &config.bins {
            let (name, force) = match bin {
                BinEntry::String(inner) => (inner.as_str(), false),
                BinEntry::Object(cfg) => {
                    if cfg.local && env.ci {
                        continue;
                    }

                    (cfg.bin.as_str(), cfg.force)
                }
            };

            let (module, version) = name.split_once('@').unwrap_or((name, "latest"));

            if !force && is_bin_installed(env, input.globals_dir.as_ref(), module, version) {
                continue;
            }

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
                    .cache(CacheStrategy::Memory)
                    .label(format!("go-bins-{version}")),
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
