//! Contributing .NET structure to moon's project graph.
//!
//! `extend_project_graph` runs in three passes: index every project's MSBuild
//! files, evaluate them all in one batched MSBuild invocation, then map each
//! project's `ProjectReference` items onto moon project ids and infer its tasks.

use crate::config::DotnetToolchainConfig;
use crate::discovery::{find_lock_files, find_project_files};
use crate::eval_cache::write_eval_cache;
use crate::infer_tasks::{InferInputs, infer_tasks, reportable_conflicts};
use crate::inherited_tasks::load_inherited_task_ids;
use crate::msbuild::{
    EvalEnv, MsbuildEvaluation, common_source_prefix, evaluate_project, evaluate_projects_batch,
    is_sdk_resolution_failure, normalize_path_key,
};
use crate::tier2_env::{
    build_eval_env, find_sdk_requirement, sdk_install_configured, uses_test_platform_runner,
};
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk::{
    HostLogInput, HostLogTarget, command_exists, get_host_environment, host_log,
    parse_toolchain_config, plugin_err,
};
use moon_pdk_api::*;
use std::collections::{BTreeMap, BTreeSet};

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

/// Everything the first pass discovers: which MSBuild files each moon project
/// owns, plus two indexes for resolving a `ProjectReference` path back to a moon
/// project id.
struct ProjectIndexes {
    files: BTreeMap<Id, Vec<VirtualPath>>,

    /// Normalized host-real project-file path -> owning project.
    by_real_path: BTreeMap<String, Id>,

    /// Normalized workspace-relative `/<source>/<file>` suffix -> owning
    /// project, or `None` when two projects share the suffix (ambiguous).
    ///
    /// This exists because the exact real paths can differ lexically from what
    /// MSBuild prints — Windows 8.3 short names such as `RUNNER~1` in a temp-dir
    /// prefix expand to their long form in MSBuild output.
    by_suffix: BTreeMap<String, Option<Id>>,
}

/// Pass 1: locate every project's MSBuild files and index them.
fn build_project_indexes(input: &ExtendProjectGraphInput) -> ProjectIndexes {
    let mut indexes = ProjectIndexes {
        files: BTreeMap::new(),
        by_real_path: BTreeMap::new(),
        by_suffix: BTreeMap::new(),
    };

    for (id, source) in &input.project_sources {
        let project_root = input.context.workspace_root.join(source);
        let files = find_project_files(&project_root);

        if files.is_empty() {
            // Not a .NET project; none of our business.
            continue;
        }

        for file in &files {
            if let Some(real) = file.real_path() {
                indexes
                    .by_real_path
                    .insert(normalize_path_key(&real.to_string_lossy()), id.to_owned());
            }

            if let Some(name) = file.file_name().and_then(|name| name.to_str()) {
                let source = source.trim_matches('/');
                let suffix = if source.is_empty() || source == "." {
                    normalize_path_key(&format!("/{name}"))
                } else {
                    normalize_path_key(&format!("/{source}/{name}"))
                };

                indexes
                    .by_suffix
                    .entry(suffix)
                    .and_modify(|existing| *existing = None)
                    .or_insert_with(|| Some(id.to_owned()));
            }
        }

        indexes.files.insert(id.to_owned(), files);
    }

    indexes
}

/// Resolve a `ProjectReference` path to the moon project that owns it: exact
/// real-path match first, then the **longest** matching workspace-relative
/// suffix.
///
/// Longest, not first. One key can end with several indexed suffixes: with
/// sources `lib` and `src/lib` both holding an `App.csproj`, a reference to
/// `/ws/src/lib/App.csproj` ends with both `/lib/app.csproj` and
/// `/src/lib/app.csproj`. Taking the first match in `BTreeMap` order returned
/// the lexicographically smaller `/lib/...`, i.e. a dependency edge pointing at
/// the wrong project — the worst failure mode here, since the graph is then
/// silently wrong rather than merely incomplete.
///
/// There is no tie to break: two suffixes of equal length that are both
/// suffixes of the same key are the same string, and the index holds each key
/// once. Genuinely ambiguous suffixes are already recorded as `None` when the
/// index is built.
fn resolve_reference<'index>(
    indexes: &'index ProjectIndexes,
    reference: &str,
) -> Option<&'index Id> {
    let key = normalize_path_key(reference);

    indexes.by_real_path.get(&key).or_else(|| {
        indexes
            .by_suffix
            .iter()
            .filter(|(suffix, id)| id.is_some() && key.ends_with(suffix.as_str()))
            .max_by_key(|(suffix, _)| suffix.len())
            .and_then(|(_, id)| id.as_ref())
    })
}

/// The working directory to evaluate from: the deepest directory containing
/// every .NET project, so a `global.json` in that subtree governs evaluation
/// exactly as it governs the tasks that run inside it. Without an explicit
/// working directory the dotnet host would resolve `global.json` from wherever
/// moon happened to be invoked, so the same workspace could evaluate under
/// different SDKs run to run.
fn batch_eval_env(
    config: &DotnetToolchainConfig,
    input: &ExtendProjectGraphInput,
    indexes: &ProjectIndexes,
) -> AnyResult<EvalEnv> {
    let sources = input
        .project_sources
        .iter()
        .filter(|(id, _)| indexes.files.contains_key(*id))
        .map(|(_, source)| source.as_str())
        .collect::<Vec<_>>();

    let eval_prefix = common_source_prefix(&sources);
    let eval_dir = if eval_prefix.is_empty() {
        input.context.workspace_root.clone()
    } else {
        input.context.workspace_root.join(&eval_prefix)
    };

    build_eval_env(config, eval_dir, &input.context.workspace_root)
}

/// Pass 2: evaluate every project in a single batched MSBuild invocation (one
/// process, parallel in-process evaluation). The dotnet/MSBuild startup cost
/// dominates per-project evaluation, so this is the difference between minutes
/// and seconds on large workspaces.
///
/// A recoverable batch failure yields an empty map: each project then falls back
/// to its own evaluation, keeping the batch purely an optimization.
///
/// An unresolvable SDK is different — it dooms every project, so falling back
/// would only repeat the host's cryptic output once per project and still leave
/// the graph empty. It is reported once, naming the pin and the ways out, and
/// whether that report is fatal depends on whether anything is going to fix it:
///
/// - No `version:` configured: nothing will install the missing SDK, so this is a
///   terminal misconfiguration and the graph build fails with the guidance.
/// - `version:` configured: tier 3 installs that SDK later in the same run — the
///   project graph is built before the action pipeline starts, so failing here
///   would deadlock the very bootstrap the setting exists for. Warns instead and
///   returns `None`, the same way a missing `dotnet` does.
///
/// `None` means "contribute nothing at all", and is distinct from `Some(empty)`:
/// an empty batch sends every project through per-project evaluation, which is
/// right for a recoverable failure and wrong when no SDK exists — there it would
/// reproduce the host's output once per project, the exact noise this reports
/// once instead.
fn run_batch_evaluation(
    input: &ExtendProjectGraphInput,
    indexes: &ProjectIndexes,
    eval_env: &EvalEnv,
) -> FnResult<Option<BTreeMap<String, MsbuildEvaluation>>> {
    let all_project_paths = indexes
        .files
        .values()
        .flatten()
        .filter_map(|file| file.real_path())
        .collect::<Vec<_>>();

    match evaluate_projects_batch(&input.context.workspace_root, &all_project_paths, eval_env) {
        Ok(results) => Ok(Some(results)),
        Err(error) => {
            let message = error.to_string();

            if is_sdk_resolution_failure(&message) {
                let pin = find_sdk_requirement(
                    eval_env
                        .cwd
                        .as_ref()
                        .unwrap_or(&input.context.workspace_root),
                    &input.context.workspace_root,
                );

                let requirement = match &pin {
                    Some((file, requirement)) => format!(
                        "The .NET SDK pinned by <path>{}</path> (<symbol>{}</symbol>) is not available",
                        file, requirement.version
                    ),
                    None => "No usable .NET SDK was found".to_owned(),
                };

                if sdk_install_configured(&input.context.workspace_root) {
                    host_log!(
                        warn,
                        "{requirement} yet, so .NET project graph evaluation is being skipped — no dependency edges or inferred tasks will be contributed on this run. moon installs the SDK configured by <property>version</property> later in this run; re-run afterwards to pick them up."
                    );

                    return Ok(None);
                }

                return Err(plugin_err!(
                    "{requirement}, so MSBuild evaluation cannot run.\n\nInstall that SDK, set <property>version</property> under <property>dotnet</property> in <file>.moon/toolchains.yml</file> to have moon install it, or point <property>dotnetRoot</property> at an SDK that satisfies the pin.\n\n{message}"
                ));
            }

            host_log!(
                warn,
                "Batched MSBuild evaluation failed; falling back to per-project evaluation: {}",
                error
            );

            Ok(Some(BTreeMap::new()))
        }
    }
}

/// Task ids inference must not contribute, because an inherited task file
/// already defines them. moon merges plugin tasks over inherited ones with
/// args-append semantics, which produces a garbage command. Project-level
/// `moon.yml` needs no such handling: moon guarantees local tasks win.
fn reserved_task_ids(
    config: &DotnetToolchainConfig,
    workspace_root: &VirtualPath,
) -> AnyResult<BTreeSet<String>> {
    let reserved = load_inherited_task_ids(workspace_root);

    // Report once per workspace, not once per project: without this, "no
    // project has a build task" has no visible cause.
    for (task_id, file) in reportable_conflicts(&reserved, &config.infer_tasks) {
        host_log!(
            warn,
            "Not inferring the <id>{}</id> task: <path>{}</path> already defines it, and moon merges inherited and plugin tasks by appending args — which would produce a broken command. Rename or remove that task to let inference contribute, or list only the tasks you want in <property>inferTasks</property>.",
            task_id,
            file
        );
    }

    Ok(reserved.into_keys().collect())
}

/// Shared, read-only state for mapping one project.
struct GraphContext<'a> {
    config: &'a DotnetToolchainConfig,
    indexes: &'a ProjectIndexes,
    eval_env: &'a EvalEnv,
    reserved_task_ids: &'a BTreeSet<String>,
    workspace_root: &'a VirtualPath,

    /// Host-real workspace root, for making evaluated output paths relative.
    workspace_dir: &'a str,

    infer_tasks_enabled: bool,
}

/// What one moon project contributed.
struct ProjectEvaluation {
    output: ExtendProjectOutput,
    packages: BTreeMap<String, String>,

    /// Every one of the project's MSBuild files evaluated successfully. The
    /// package set above may only be cached when this holds — a partial set is
    /// indistinguishable from a complete one once written, and would then be
    /// served under a digest that stays valid.
    complete: bool,

    /// Project files to report to moon as graph inputs.
    input_files: Vec<std::path::PathBuf>,
}

/// Pass 3, for one project: map its `ProjectReference` items onto moon project
/// ids, take its alias from the evaluated `AssemblyName`, and infer its tasks.
fn extend_one_project(
    ctx: &GraphContext<'_>,
    id: &Id,
    files: &[VirtualPath],
    batch: &mut BTreeMap<String, MsbuildEvaluation>,
    test_platform_runner: bool,
) -> AnyResult<ProjectEvaluation> {
    let mut result = ProjectEvaluation {
        output: ExtendProjectOutput::default(),
        packages: BTreeMap::new(),
        complete: true,
        input_files: vec![],
    };

    let mut seen_deps: BTreeSet<Id> = BTreeSet::new();

    for file in files {
        let Some(real_path) = file.real_path() else {
            result.complete = false;
            continue;
        };

        let batch_key = normalize_path_key(&real_path.to_string_lossy());

        let evaluation = if let Some(evaluation) = batch.remove(&batch_key) {
            evaluation
        } else {
            // Fall back with the project's own directory as the working
            // directory — the same `global.json` its tasks will resolve.
            let single_env = EvalEnv {
                cwd: file.parent().or_else(|| ctx.eval_env.cwd.clone()),
                ..ctx.eval_env.clone()
            };

            match evaluate_project(&real_path, &single_env) {
                Ok(evaluation) => evaluation,
                Err(error) => {
                    // One broken project must not take down graph construction
                    // for the whole workspace.
                    host_log!(
                        warn,
                        "MSBuild evaluation failed for project <id>{}</id> ({}): {}",
                        id,
                        real_path.display(),
                        error
                    );

                    result.complete = false;
                    continue;
                }
            }
        };

        result.packages.extend(evaluation.package_references());

        // Project alias from the evaluated AssemblyName, so tasks can reference
        // the project by its .NET name (e.g. `moon run MyCompany.App:build`).
        // moon silently skips aliases that collide with project ids or
        // already-claimed aliases, and an alias equal to its own id is a no-op —
        // no need to filter beyond emptiness here.
        if result.output.alias.is_none() {
            let assembly_name = evaluation.property("AssemblyName");

            if !assembly_name.is_empty() {
                result.output.alias = Some(assembly_name.to_owned());
            }
        }

        if ctx.config.infer_dependencies {
            for reference in evaluation.project_reference_paths() {
                let Some(dep_id) = resolve_reference(ctx.indexes, &reference) else {
                    host_log!(
                        debug,
                        "Project <id>{}</id> references {} which is outside the moon workspace; skipping",
                        id,
                        reference
                    );
                    continue;
                };

                if dep_id != id && seen_deps.insert(dep_id.to_owned()) {
                    let file_name = std::path::Path::new(&reference)
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or(reference.clone());

                    result.output.dependencies.push(ProjectDependency {
                        id: dep_id.to_owned(),
                        scope: DependencyScope::Production,
                        via: Some(format!("project-reference {file_name}")),
                    });
                }
            }
        }

        if ctx.infer_tasks_enabled {
            let project_dir = real_path
                .parent()
                .map(|dir| dir.to_string_lossy().to_string())
                .unwrap_or_default();

            // Bare `dotnet build` errors on ambiguity when the directory holds
            // several project files — pass the file explicitly.
            let explicit_project_file = if files.len() > 1 {
                file.file_name().and_then(|name| name.to_str())
            } else {
                None
            };

            let inferred = infer_tasks(
                &ctx.config.infer_tasks,
                ctx.reserved_task_ids,
                &InferInputs {
                    evaluation: &evaluation,
                    explicit_project_file,
                    project_dir: &project_dir,
                    workspace_dir: ctx.workspace_dir,
                    test_platform_runner,
                },
            );

            match inferred {
                Ok(tasks) => {
                    for (task_id, task) in tasks {
                        result.output.tasks.entry(task_id).or_insert(task);
                    }
                }
                Err(error) => {
                    host_log!(
                        warn,
                        "Task inference failed for project <id>{}</id>: {}",
                        id,
                        error
                    );
                }
            }
        }

        if let Some(virtual_file) = file.virtual_path() {
            result.input_files.push(virtual_file);
        }
    }

    Ok(result)
}

#[plugin_fn]
pub fn extend_project_graph(
    Json(mut input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    // Taken rather than moved out, so `input` stays whole for the helpers below.
    let config = parse_toolchain_config::<DotnetToolchainConfig>(std::mem::take(
        &mut input.toolchain_config,
    ))?;
    let mut output = ExtendProjectGraphOutput::default();

    let infer_tasks_enabled = config.infer_tasks.any_enabled();

    if !config.infer_dependencies && !infer_tasks_enabled {
        return Ok(Json(output));
    }

    let indexes = build_project_indexes(&input);

    if indexes.files.is_empty() {
        return Ok(Json(output));
    }

    // Degrade rather than fail, like `parse_manifest` and `hash_task_contents`
    // below. The graph is built before the action pipeline runs, so a `version:`
    // configured for tier 3 to install has not been installed yet on a fresh
    // machine — erroring here would fail the whole-workspace graph, for every
    // toolchain, before moon ever gets to install the SDK it was told to
    // install.
    if !command_exists(&get_host_environment()?, "dotnet") {
        host_log!(
            warn,
            "No <symbol>dotnet</symbol> executable found on PATH, skipping .NET project graph evaluation — no dependency edges or inferred tasks will be contributed. Install a .NET 8+ SDK, or set <property>version</property> in <file>.moon/toolchains.yml</file> to have moon install one."
        );

        return Ok(Json(output));
    }

    let eval_env = batch_eval_env(&config, &input, &indexes)?;

    // `None` means no SDK is available to evaluate with. Returning here rather
    // than continuing with an empty batch is what keeps a single unresolvable pin
    // from being reported once per project by the per-project fallback.
    let Some(mut batch) = run_batch_evaluation(&input, &indexes, &eval_env)? else {
        return Ok(Json(output));
    };

    let workspace_dir = input
        .context
        .workspace_root
        .real_path()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();

    let reserved = if infer_tasks_enabled {
        reserved_task_ids(&config, &input.context.workspace_root)?
    } else {
        BTreeSet::new()
    };

    let ctx = GraphContext {
        config: &config,
        indexes: &indexes,
        eval_env: &eval_env,
        reserved_task_ids: &reserved,
        workspace_root: &input.context.workspace_root,
        workspace_dir: &workspace_dir,
        infer_tasks_enabled,
    };

    for (id, files) in &indexes.files {
        let project_root = input
            .project_sources
            .get(id)
            .map(|source| input.context.workspace_root.join(source));

        // Which `dotnet test` flavour this project's tasks will run under.
        let test_platform_runner = infer_tasks_enabled
            && project_root
                .as_ref()
                .is_some_and(|root| uses_test_platform_runner(root, ctx.workspace_root));

        let result = extend_one_project(&ctx, id, files, &mut batch, test_platform_runner)?;

        output.input_files.extend(result.input_files);

        // Hand the evaluated package set to task hashing. Projects with a lock
        // file take the lock-file branch there and never need it.
        if result.complete
            && let Some(project_root) = project_root
            && find_lock_files(&project_root).is_empty()
        {
            write_eval_cache(
                &input.context.workspace_root,
                id.as_str(),
                &project_root,
                &ctx.config.msbuild_properties,
                result.packages,
            );
        }

        if !result.output.dependencies.is_empty()
            || !result.output.tasks.is_empty()
            || result.output.alias.is_some()
        {
            output
                .extended_projects
                .insert(id.to_owned(), result.output);
        }
    }

    Ok(Json(output))
}
