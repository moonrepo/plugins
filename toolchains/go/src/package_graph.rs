use crate::config::GoToolchainConfig;
use crate::go_mod::{GoMod, ModuleReplacement, Replacement, parse_go_mod};
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk::{HostLogInput, HostLogTarget, exec, host_log};
use moon_pdk_api::*;
use serde::Deserialize;
use starbase_utils::fs;
use std::collections::{BTreeMap, BTreeSet};

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

/// A package from `go list -json`, with only the fields that are requested.
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
struct GoListPackage {
    dir: String,
    module: Option<GoListModule>,
    go_files: Vec<String>,
    cgo_files: Vec<String>,
    test_go_files: Vec<String>,
    x_test_go_files: Vec<String>,
    imports: Vec<String>,
    test_imports: Vec<String>,
    x_test_imports: Vec<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
struct GoListModule {
    dir: String,
}

// The `go list` half of the package graph: the packages matching `patterns`
// in one module, with their direct imports. `-e` reports packages that fail
// to load, like an import that can't be resolved, instead of failing.
fn execute_go_list(
    dir: &VirtualPath,
    patterns: &BTreeSet<String>,
) -> AnyResult<Vec<GoListPackage>> {
    let mut args = vec![
        "list".to_owned(),
        "-e".into(),
        "-json=Dir,Module,GoFiles,CgoFiles,TestGoFiles,XTestGoFiles,Imports,TestImports,XTestImports"
            .into(),
    ];
    args.extend(patterns.iter().cloned());

    let result = exec(ExecCommandInput::pipe("go", args).cwd(dir.to_owned()))?;

    // ADR-0007.3 exception: parse the packages that `go list -e` could load, even when it fails
    if result.exit_code != 0 {
        host_log!(
            warn,
            "Failed to infer Go relationships for <path>{}</path>: {}",
            dir,
            result.stderr.lines().next().unwrap_or_default()
        );
    }

    // A stream of JSON objects, one per package
    Ok(json::Deserializer::from_str(&result.stdout)
        .into_iter::<GoListPackage>()
        .map_while(Result::ok)
        .collect())
}

// Patterns are relative to each project, while `go list` runs from the root
// of its module.
fn scope_pattern(relative: &str, pattern: &str) -> String {
    if relative.is_empty() || !pattern.starts_with('.') {
        pattern.to_owned()
    } else if pattern == "." {
        format!("./{relative}")
    } else {
        format!("./{relative}/{}", pattern.trim_start_matches("./"))
    }
}

// Whether a workspace-relative `dir` is the `parent` dir itself, or nested
// beneath it.
fn dir_within(dir: &str, parent: &str) -> bool {
    parent.is_empty()
        || dir == parent
        || dir
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('/'))
}

// Whether `import_path` is the package at `prefix` itself, or nested beneath
// it.
fn import_within(import_path: &str, prefix: &str) -> bool {
    import_path == prefix
        || import_path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

// Workspace-relative dirs that may own a directory's `go.mod`: the dir
// itself, then each ancestor up to the workspace root ("").
pub fn module_dir_candidates(source: &str) -> Vec<&str> {
    let mut candidates = vec![];
    let mut dir = source;

    while !dir.is_empty() {
        candidates.push(dir);
        dir = dir.rfind('/').map_or("", |index| &dir[..index]);
    }

    candidates.push("");
    candidates
}

// Resolves workspace directories to the import paths their packages live
// under, caching each `go.mod` it parses along the way. A `None` cache entry
// memoizes "no usable `go.mod` here", so ancestors shared between
// directories only hit the disk once.
struct ModuleResolver {
    workspace_root: VirtualPath,
    go_mods: BTreeMap<String, Option<GoMod>>,
}

impl ModuleResolver {
    fn new(workspace_root: VirtualPath) -> Self {
        Self {
            workspace_root,
            go_mods: BTreeMap::default(),
        }
    }

    // The canonical import path of a workspace-relative dir: the module path
    // of the nearest `go.mod` at or above it (bounded by the workspace
    // root), joined with the dir's path relative to that module root.
    // Returns the owning dir alongside so callers can key back into the
    // cache.
    fn import_path(&mut self, source: &str) -> AnyResult<Option<(String, String)>> {
        for dir in module_dir_candidates(source) {
            if !self.go_mods.contains_key(dir) {
                let go_mod_path = self.go_mod_path(dir);

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

    // The parsed manifest owned by a dir previously resolved through
    // `import_path`, with any major version suffix already applied to its
    // module path.
    fn manifest(&self, dir: &str) -> Option<&GoMod> {
        self.go_mods.get(dir).and_then(|entry| entry.as_ref())
    }

    fn go_mod_path(&self, dir: &str) -> VirtualPath {
        self.module_file_path(dir, "go.mod")
    }

    fn go_sum_path(&self, dir: &str) -> VirtualPath {
        self.module_file_path(dir, "go.sum")
    }

    // A file sibling to a module's `go.mod`, at the workspace root when `dir`
    // is empty.
    fn module_file_path(&self, dir: &str, file: &str) -> VirtualPath {
        if dir.is_empty() {
            self.workspace_root.join(file)
        } else {
            self.workspace_root.join(dir).join(file)
        }
    }
}

pub struct GoProject {
    pub id: Id,
    /// Module path as declared when the project owns its `go.mod`
    pub alias: Option<String>,
    root: VirtualPath,
    /// Workspace-relative dir, empty for the workspace root
    source: String,
    /// Workspace-relative dir of the module the project belongs to
    module_dir: Option<String>,
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
pub struct GoPackageGraph {
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
    /// Direct imports of each project's packages, from `go list`
    imports: BTreeMap<Id, ProjectImports>,
    /// The Go files `go list` read imports from, so that a changed import
    /// refreshes the graph, keyed by path for a stable order
    source_files: BTreeMap<String, VirtualPath>,
}

#[derive(Default)]
struct ProjectImports {
    production: BTreeSet<String>,
    /// From tests, including external `_test` packages
    development: BTreeSet<String>,
}

impl GoPackageGraph {
    pub fn new(workspace_root: VirtualPath, config: GoToolchainConfig, go_exists: bool) -> Self {
        Self {
            resolver: ModuleResolver::new(workspace_root.clone()),
            workspace_root,
            config,
            go_exists,
            projects: vec![],
            source_to_id: BTreeMap::default(),
            package_prefixes: vec![],
            input_files: vec![],
            imports: BTreeMap::default(),
            source_files: BTreeMap::default(),
        }
    }

    // First pass: resolve every project to its import path and manifest, and
    // build the prefix map that imports are resolved against.
    pub fn load_projects(&mut self, sources: BTreeMap<Id, String>) -> AnyResult<()> {
        for (id, source) in sources {
            let root = self.workspace_root.join(&source);
            let source = if source == "." { "" } else { source.as_str() };

            self.source_to_id.insert(source.to_owned(), id.clone());

            let mut project = GoProject {
                id,
                root,
                source: source.to_owned(),
                module_dir: None,
                alias: None,
                import_path: None,
                requires: vec![],
            };

            if let Some((mod_dir, import_path)) = self.resolver.import_path(source)? {
                let go_mod_path = self.resolver.go_mod_path(&mod_dir);
                self.add_input_file(go_mod_path);

                // `go.sum` sits beside `go.mod`; a changed checksum set (a
                // dependency added, upgraded, or dropped) must invalidate the
                // resolved graph too. Absent for a module with no dependencies.
                let go_sum_path = self.resolver.go_sum_path(&mod_dir);
                if go_sum_path.exists() {
                    self.add_input_file(go_sum_path);
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
                project.module_dir = Some(mod_dir);
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

        if self.go_exists
            && (self.config.infer_relationships || self.config.infer_relationships_from_tests)
        {
            self.load_imports()?;
        }

        Ok(())
    }

    // Runs `go list` once per module, for the packages of every project in
    // it, and assigns each package to the project whose dir contains it.
    fn load_imports(&mut self) -> AnyResult<()> {
        let patterns = if self.config.infer_relationships_packages.is_empty() {
            // All packages, including those in subdirectories (internal/, pkg/, ...)
            vec!["./...".to_owned()]
        } else {
            self.config.infer_relationships_packages.clone()
        };

        let mut modules = BTreeMap::<String, BTreeSet<String>>::default();

        for project in &self.projects {
            if let Some(module_dir) = &project.module_dir {
                let relative = project.source[module_dir.len()..].trim_start_matches('/');

                modules.entry(module_dir.to_owned()).or_default().extend(
                    patterns
                        .iter()
                        .map(|pattern| scope_pattern(relative, pattern)),
                );
            }
        }

        let with_tests = self.config.infer_relationships_from_tests;

        for (module_dir, patterns) in modules {
            for package in execute_go_list(&self.workspace_path(&module_dir), &patterns)? {
                // Relative to the module from Go's own paths, as it may have
                // resolved symlinks that the workspace path still has
                let Some(relative) = package
                    .module
                    .as_ref()
                    .and_then(|module| package.dir.strip_prefix(&module.dir))
                else {
                    continue;
                };

                let relative = relative.replace('\\', "/");
                let relative = relative.trim_matches('/');
                let source = match (module_dir.is_empty(), relative.is_empty()) {
                    (true, _) => relative.to_owned(),
                    (false, true) => module_dir.clone(),
                    (false, false) => format!("{module_dir}/{relative}"),
                };

                let Some(owner) = self.resolve_source(&source).cloned() else {
                    continue;
                };

                let imports = self.imports.entry(owner).or_default();
                imports.production.extend(package.imports);
                imports.development.extend(package.test_imports);
                imports.development.extend(package.x_test_imports);

                let mut files = [package.go_files, package.cgo_files].concat();

                if with_tests {
                    files.extend(package.test_go_files);
                    files.extend(package.x_test_go_files);
                }

                let dir = self.workspace_path(&source);

                for file in files {
                    let path = dir.join(&file);

                    self.source_files.insert(path.to_string(), path);
                }
            }
        }

        Ok(())
    }

    // Resolves a workspace-relative dir to the project that contains it.
    // The longest match wins, so a parent project can't claim the packages
    // of projects nested beneath it.
    fn resolve_source(&self, dir: &str) -> Option<&Id> {
        self.source_to_id
            .iter()
            .filter(|(source, _)| dir_within(dir, source))
            .max_by_key(|(source, _)| source.len())
            .map(|(_, id)| id)
    }

    fn workspace_path(&self, dir: &str) -> VirtualPath {
        if dir.is_empty() {
            self.workspace_root.clone()
        } else {
            self.workspace_root.join(dir)
        }
    }

    pub fn projects(&self) -> &[GoProject] {
        &self.projects
    }

    pub fn into_input_files(self) -> Vec<VirtualPath> {
        let mut files = self.input_files;
        files.extend(self.source_files.into_values());
        files
    }

    fn add_input_file(&mut self, path: VirtualPath) {
        if !self.input_files.contains(&path) {
            self.input_files.push(path);
        }
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
    pub fn project_dependencies(&self, project: &GoProject) -> AnyResult<Vec<ProjectDependency>> {
        let mut dependencies = self.dependencies_from_imports(project);

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

    fn dependencies_from_imports(&self, project: &GoProject) -> Vec<ProjectDependency> {
        let mut dependencies = vec![];
        let mut seen = BTreeSet::new();

        let Some(imports) = self.imports.get(&project.id) else {
            return dependencies;
        };

        for (enabled, imports, scope) in [
            (
                self.config.infer_relationships,
                &imports.production,
                DependencyScope::Production,
            ),
            (
                self.config.infer_relationships_from_tests,
                &imports.development,
                DependencyScope::Development,
            ),
        ] {
            if !enabled {
                continue;
            }

            for import_path in imports {
                // Imports of the project's own packages, like an external
                // `_test` package importing the package under test
                if let Some(own) = project.import_path.as_deref()
                    && import_within(import_path, own)
                {
                    continue;
                }

                if let Some(dep_id) = self.resolve_import(import_path)
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

        dependencies
    }
}

// Lexically resolve a relative path (`./`, `../`) against a workspace
// relative source, returning `None` when it escapes the workspace
pub fn resolve_source_path(base: &str, path: &str) -> Option<String> {
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
pub fn is_version_segment(segment: &str) -> bool {
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
