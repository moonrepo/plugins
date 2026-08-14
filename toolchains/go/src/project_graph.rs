use crate::config::GoToolchainConfig;
use crate::go_mod::{GoMod, ModuleReplacement, Replacement, parse_go_mod};
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk::{command_exists, exec, get_host_environment, parse_toolchain_config_schema};
use moon_pdk_api::*;
use starbase_utils::fs;
use std::collections::{BTreeMap, BTreeSet};

fn execute_go_list(dir: &VirtualPath, packages: &[String], test: bool) -> AnyResult<Vec<String>> {
    let mut args = vec![
        "list",
        "-deps",
        "-f",
        "{{if .Module}}{{.ImportPath}}{{end}}",
    ];

    if test {
        args.push("-test");
    }

    // Scan all packages recursively by default so that dependencies imported
    // only from subdirectories (internal/, pkg/, ...) are also inferred.
    if packages.is_empty() {
        args.push("./...");
    } else {
        for package in packages {
            args.push(package.as_str());
        }
    }

    let result = exec(ExecCommandInput::pipe("go", args).cwd(dir.to_owned()))?;

    if result.exit_code != 0 {
        return Ok(vec![]);
    }

    Ok(result
        .stdout
        .lines()
        .filter_map(|line| {
            // Test binary pseudo-packages render as `pkg [pkg.test]`;
            // only the real package path participates in matching.
            let import_path = line.trim().split(' ').next().unwrap_or_default();

            (!import_path.is_empty()).then(|| import_path.to_owned())
        })
        .collect())
}

// `go.mod` manifests parsed while resolving project import paths, keyed by
// workspace-relative dir. A `None` entry memoizes "no usable `go.mod` here",
// so ancestors shared between projects only hit the disk once.
type GoModCache = BTreeMap<String, Option<GoMod>>;

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

// Whether `import_path` is the package at `prefix` itself, or nested beneath
// it.
fn import_within(import_path: &str, prefix: &str) -> bool {
    import_path == prefix
        || import_path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

// Workspace-relative dirs that may own a project's `go.mod`: the project dir
// itself, then each ancestor up to the workspace root ("").
fn module_dir_candidates(source: &str) -> Vec<&str> {
    let mut candidates = vec![];
    let mut dir = source;

    while !dir.is_empty() {
        candidates.push(dir);
        dir = dir.rfind('/').map_or("", |index| &dir[..index]);
    }

    candidates.push("");
    candidates
}

// All the state relationship inference works from: the resolved projects,
// the import-path prefix map they resolve against, and the `go.mod` cache
// used to build both.
struct GoProjectGraph {
    workspace_root: VirtualPath,
    config: GoToolchainConfig,
    go_exists: bool,
    go_mods: GoModCache,
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

            if let Some((mod_dir, import_path)) = self.project_import_path(source)? {
                let go_mod_path = if mod_dir.is_empty() {
                    self.workspace_root.join("go.mod")
                } else {
                    self.workspace_root.join(&mod_dir).join("go.mod")
                };

                if !self.input_files.contains(&go_mod_path) {
                    self.input_files.push(go_mod_path);
                }

                // An ancestor's requires describe the whole module, so they
                // only participate for the project that owns the `go.mod`
                if mod_dir == source
                    && let Some(manifest) =
                        self.go_mods.get(&mod_dir).and_then(|entry| entry.as_ref())
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

    // The canonical import path of a project: the module path of the nearest
    // `go.mod` at or above the project dir (bounded by the workspace root),
    // joined with the project's path relative to that module root. This is
    // what makes relationships resolvable when many projects share a single
    // module. Returns the owning dir alongside so callers can key back into
    // the cache.
    fn project_import_path(&mut self, source: &str) -> AnyResult<Option<(String, String)>> {
        for dir in module_dir_candidates(source) {
            if !self.go_mods.contains_key(dir) {
                let go_mod_path = if dir.is_empty() {
                    self.workspace_root.join("go.mod")
                } else {
                    self.workspace_root.join(dir).join("go.mod")
                };

                let manifest = if go_mod_path.exists() {
                    Some(parse_go_mod(fs::read_file(&go_mod_path)?)?)
                        .filter(|manifest| !manifest.module.is_empty())
                        .map(|mut manifest| {
                            // A module in a major version folder (v2+) is
                            // imported with the version suffix, even when the
                            // `module` directive omits it
                            // https://go.dev/ref/mod#major-version-suffixes
                            if let Some(version) = dir
                                .rsplit('/')
                                .next()
                                .filter(|segment| is_version_segment(segment))
                                && !is_version_segment(
                                    manifest.module.rsplit('/').next().unwrap_or_default(),
                                )
                            {
                                manifest.module = format!("{}/{version}", manifest.module);
                            }

                            manifest
                        })
                } else {
                    None
                };

                self.go_mods.insert(dir.to_owned(), manifest);
            }

            if let Some(manifest) = self.go_mods.get(dir).and_then(|entry| entry.as_ref()) {
                let relative = source[dir.len()..].trim_start_matches('/');

                let import_path = if relative.is_empty() {
                    manifest.module.clone()
                } else {
                    format!("{}/{relative}", manifest.module)
                };

                return Ok(Some((dir.to_owned(), import_path)));
            }
        }

        Ok(None)
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
        workspace_root: input.context.workspace_root,
        config,
        go_exists: command_exists(env, "go"),
        go_mods: GoModCache::default(),
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

// Lexically resolve a relative path (`./`, `../`) against a workspace
// relative source, returning `None` when it escapes the workspace
fn resolve_source_path(base: &str, path: &str) -> Option<String> {
    if path.starts_with('/') {
        return None;
    }

    let mut segments = base
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>();

    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            _ => segments.push(segment),
        }
    }

    Some(segments.join("/"))
}

// A major version suffix segment: v2, v3, etc, but never v0 or v1
// https://go.dev/ref/mod#major-version-suffixes
pub(crate) fn is_version_segment(segment: &str) -> bool {
    match segment.strip_prefix('v') {
        Some(digits) => {
            !digits.is_empty()
                && !digits.starts_with('0')
                && digits != "1"
                && digits.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}
