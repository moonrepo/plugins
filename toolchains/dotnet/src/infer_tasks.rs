use crate::config::{INFERABLE_TASKS, InferTasksSetting};
use crate::msbuild::MsbuildEvaluation;
use moon_common::Id;
use moon_config::{
    Input, Output, PartialTaskArgs, PartialTaskConfig, PartialTaskDependency,
    PartialTaskDependencyConfig, PartialTaskOptionsConfig, TaskOptionCache, TaskOptionRunInCI,
};
use moon_pdk_api::{AnyResult, anyhow};
use moon_target::Target;
use std::collections::{BTreeMap, BTreeSet};

/// Everything task inference needs to know about one MSBuild project.
pub struct InferInputs<'a> {
    pub evaluation: &'a MsbuildEvaluation,

    /// Project file name to pass explicitly in commands when the project
    /// directory holds more than one MSBuild project file (bare `dotnet
    /// build` would otherwise error on ambiguity).
    pub explicit_project_file: Option<&'a str>,

    /// Host-real absolute path of the project directory (for making
    /// evaluated output paths project-relative). Forward or back slashes.
    pub project_dir: &'a str,

    /// Host-real absolute path of the workspace root.
    pub workspace_dir: &'a str,

    /// Whether the `global.json` governing this project selects
    /// Microsoft.Testing.Platform for `dotnet test`. A project can also opt
    /// in on its own via `TestingPlatformDotnetTestSupport`, which is read
    /// from the evaluation.
    pub test_platform_runner: bool,
}

/// Strip `base` (plus one separator) from the start of `value`,
/// case-insensitively — Windows paths are case-insensitive and MSBuild
/// output casing is not guaranteed to match what moon reports. Both inputs
/// must already use forward slashes.
fn strip_prefix_ci<'a>(value: &'a str, base: &str) -> Option<&'a str> {
    if base.is_empty() {
        return None;
    }

    let mut value_iter = value.char_indices();
    let mut base_iter = base.chars();

    loop {
        let Some(base_ch) = base_iter.next() else {
            // Base fully consumed: the next value char must be the separator.
            return match value_iter.next() {
                Some((index, '/')) => Some(&value[index + 1..]),
                _ => None,
            };
        };

        let (_, value_ch) = value_iter.next()?;

        if value_ch != base_ch && !value_ch.to_lowercase().eq(base_ch.to_lowercase()) {
            return None;
        }
    }
}

/// Does a package identity mark its project as a test project?
///
/// Matching is exact or by prefix, never a substring search: real test projects
/// commonly reference `Microsoft.AspNetCore.Mvc.Testing` and
/// `Microsoft.AspNetCore.TestHost`, and plenty of non-test libraries reference
/// helpers with "test" in the name. Those must not qualify on their own.
///
/// The prefixes cover the families that replaced `Microsoft.NET.Test.Sdk` under
/// Microsoft.Testing.Platform, where that package is absent entirely — `xunit.v3`
/// ships as `xunit.v3`, `xunit.v3.core`, `xunit.v3.mtp-v2` and more, so the whole
/// family is matched rather than enumerated.
fn is_test_package(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "microsoft.net.test.sdk",
        "mstest",
        "mstest.testframework",
        "nunit3testadapter",
        "tunit",
    ];

    const PREFIXES: &[&str] = &["xunit.v3", "microsoft.testing.platform", "tunit."];

    let lower = name.to_ascii_lowercase();

    EXACT.contains(&lower.as_str()) || PREFIXES.iter().any(|prefix| lower.starts_with(prefix))
}

/// Turn an evaluated MSBuild output path into a moon task output: relative
/// paths pass through, absolute paths under the project dir become
/// project-relative, absolute paths under the workspace root become
/// workspace-relative (leading `/`). Anything else (redirected outside the
/// workspace) is `None` — the task must then disable caching rather than
/// cache the wrong directory.
pub fn resolve_output_path(raw: &str, project_dir: &str, workspace_dir: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }

    let value = raw.replace('\\', "/");
    let value = value.trim_end_matches('/');

    if value.is_empty() {
        return None;
    }

    let is_absolute = value.starts_with('/') || value.as_bytes().get(1) == Some(&b':');

    if !is_absolute {
        return Some(value.to_string());
    }

    let project_dir = project_dir.replace('\\', "/");

    if let Some(relative) = strip_prefix_ci(value, project_dir.trim_end_matches('/')) {
        return Some(relative.to_string());
    }

    let workspace_dir = workspace_dir.replace('\\', "/");

    if let Some(relative) = strip_prefix_ci(value, workspace_dir.trim_end_matches('/')) {
        return Some(format!("/{relative}"));
    }

    None
}

fn command(verb: &str, project_file: Option<&str>, extra_args: &[&str]) -> PartialTaskArgs {
    let mut list = vec!["dotnet".to_string(), verb.to_string()];

    list.extend(project_file.map(str::to_string));
    list.extend(extra_args.iter().map(|arg| arg.to_string()));

    PartialTaskArgs::List(list)
}

/// Pin the evaluated `Configuration` on cacheable commands. `dotnet build`
/// defaults to Debug but `dotnet publish` defaults to Release (.NET 8+), so
/// without an explicit `-c` a `publish --no-build` would look for outputs a
/// `build` never produced. Passing the configuration the evaluation itself
/// saw keeps every command consistent with the evaluated output paths —
/// including repos that set `Configuration` in `Directory.Build.props`.
fn pin_configuration(command: &mut PartialTaskArgs, configuration: &str) {
    if configuration.is_empty() {
        return;
    }

    if let PartialTaskArgs::List(list) = command {
        list.push("-c".into());
        list.push(configuration.into());
    }
}

fn parse_target(target: &str) -> AnyResult<PartialTaskDependency> {
    Ok(PartialTaskDependency::Target(
        Target::parse(target).map_err(|error| anyhow!("{error}"))?,
    ))
}

/// Same, but tolerated when the target does not exist. Required for `~:` deps:
/// moon defaults `optional` to `false` for the `OwnSelf` scope, so a project
/// that infers `test` or `publish` without `build` — `inferTasks: ['test']`, or
/// a `build` id claimed by an inherited task file — would fail project-graph
/// construction outright with `UnknownDepTarget` rather than simply losing the
/// ordering edge.
fn parse_optional_target(target: &str) -> AnyResult<PartialTaskDependency> {
    Ok(PartialTaskDependency::Object(PartialTaskDependencyConfig {
        target: Some(Target::parse(target).map_err(|error| anyhow!("{error}"))?),
        optional: Some(true),
        ..Default::default()
    }))
}

/// Inputs for cacheable tasks: everything in the project EXCEPT the
/// evaluated output and intermediate directories. moon's default `**/*`
/// would otherwise hash `obj/` (which MSBuild mutates on every build), so
/// task hashes would never stabilize and nothing would ever be a cache hit.
fn stable_inputs(inputs: &InferInputs) -> AnyResult<Vec<Input>> {
    let mut list = vec![Input::parse("**/*").map_err(|error| anyhow!("{error}"))?];

    for property in ["BaseOutputPath", "BaseIntermediateOutputPath"] {
        if let Some(dir) = resolve_output_path(
            inputs.evaluation.property(property),
            inputs.project_dir,
            inputs.workspace_dir,
        ) {
            list.push(Input::parse(format!("!{dir}/**")).map_err(|error| anyhow!("{error}"))?);
        }
    }

    Ok(list)
}

/// Give a task its evaluated outputs, or disable caching when they could
/// not be determined (never cache the wrong directory).
fn apply_outputs(task: &mut PartialTaskConfig, outputs: Option<String>) -> AnyResult<()> {
    match outputs {
        Some(path) => {
            task.outputs = Some(vec![
                Output::parse(&path).map_err(|error| anyhow!("{error}"))?,
            ]);
        }
        None => {
            task.options.get_or_insert_default().cache = Some(TaskOptionCache::Enabled(false));
        }
    }

    Ok(())
}

/// Task ids that inference would have contributed but had to yield to an
/// inherited task file, paired with the file that claimed each one.
///
/// Yielding is silent otherwise, which turns "why does no project have a
/// build task?" into a dead end — worth one report per workspace.
pub fn reportable_conflicts<'a>(
    reserved: &'a BTreeMap<String, String>,
    setting: &InferTasksSetting,
) -> Vec<(&'a str, &'a str)> {
    INFERABLE_TASKS
        .iter()
        .filter(|task| setting.includes(task))
        .filter_map(|task| {
            reserved
                .get_key_value(*task)
                .map(|(id, file)| (id.as_str(), file.as_str()))
        })
        .collect()
}

/// Infer `build` / `test` / `run` / `publish` tasks from one project's
/// MSBuild evaluation.
///
/// - `build` — every project; `--no-dependencies` so moon's task graph
///   (`deps: ^:build`) orchestrates upstream builds and caches each project
///   independently (verified: MSBuild resolves `ProjectReference`s from the
///   upstream `bin` output without rebuilding them).
/// - `test` — projects with `IsTestProject=true` or a `Microsoft.NET.Test.Sdk`
///   reference; `--no-build` on top of a `build` dep.
/// - `run` — `Exe`/`WinExe` non-test projects; never cached, never in CI.
/// - `publish` — `Exe`/`WinExe` non-test single-TFM projects (multi-TFM
///   `dotnet publish` requires an explicit `-f`); `--no-build` on top of a
///   `build` dep.
///
/// `restore` is deliberately NOT a task: moon models it as the
/// install-dependencies action (with `--locked-mode`), which runs before
/// tasks — hence `--no-restore` everywhere.
///
/// `reserved_ids` (task ids from applicable inherited task files) are
/// skipped entirely: moon merges plugin tasks over inherited tasks with
/// args-append semantics, which produces garbage commands — yielding is the
/// only safe move. Project-level `moon.yml` tasks need no such handling;
/// moon itself guarantees they win over plugin tasks.
pub fn infer_tasks(
    setting: &InferTasksSetting,
    reserved_ids: &BTreeSet<String>,
    inputs: &InferInputs,
) -> AnyResult<BTreeMap<Id, PartialTaskConfig>> {
    let mut tasks = BTreeMap::new();
    let evaluation = inputs.evaluation;

    // Three independent signals, because no single one covers the ecosystem:
    //
    // - `IsTestProject` comes from Microsoft.NET.Test.Sdk's build props, so it
    //   is only set once that package is restored.
    // - `IsTestingPlatformApplication` is set by test-oriented project SDKs
    //   (`<Project Sdk="MSTest.Sdk">`) without needing a restore, and by
    //   Microsoft.Testing.Platform packages once restored.
    // - The package references themselves are visible without any restore,
    //   which is the situation during a cold project-graph build.
    //
    // Verified against real repositories: an MSTest.Sdk project reports only
    // `IsTestingPlatformApplication`, an xunit.v3 project on an unrestored tree
    // reports only its package, and a BenchmarkDotNet project sets both
    // properties to `false` and must stay excluded.
    let is_test = evaluation
        .property("IsTestProject")
        .eq_ignore_ascii_case("true")
        || evaluation
            .property("IsTestingPlatformApplication")
            .eq_ignore_ascii_case("true")
        || evaluation
            .package_references()
            .keys()
            .any(|name| is_test_package(name));

    let output_type = evaluation.property("OutputType");
    let is_exe = !is_test
        && (output_type.eq_ignore_ascii_case("Exe") || output_type.eq_ignore_ascii_case("WinExe"));
    let is_single_tfm = evaluation.property("TargetFrameworks").is_empty();

    let wants = |task: &str| setting.includes(task) && !reserved_ids.contains(task);
    let file = inputs.explicit_project_file;

    let hash_inputs = stable_inputs(inputs)?;
    let configuration = evaluation.property("Configuration");

    if wants("build") {
        let mut build_command = command("build", file, &["--no-restore", "--no-dependencies"]);
        pin_configuration(&mut build_command, configuration);

        let mut task = PartialTaskConfig {
            command: Some(build_command),
            deps: Some(vec![parse_target("^:build")?]),
            description: Some(
                "Builds the project. Upstream projects build through moon task deps. (inferred)"
                    .into(),
            ),
            inputs: Some(hash_inputs.clone()),
            ..Default::default()
        };

        apply_outputs(
            &mut task,
            resolve_output_path(
                evaluation.property("BaseOutputPath"),
                inputs.project_dir,
                inputs.workspace_dir,
            ),
        )?;

        tasks.insert(Id::raw("build"), task);
    }

    if is_test && wants("test") {
        // Microsoft.Testing.Platform's `dotnet test` takes the project
        // through `--project` and rejects a positional path; classic VSTest
        // mode is the exact opposite and rejects `--project`. Both verified
        // against SDK 10.0.201, so the flavour has to match the runner.
        let uses_test_platform = inputs.test_platform_runner
            || evaluation
                .property("TestingPlatformDotnetTestSupport")
                .eq_ignore_ascii_case("true");

        let mut test_command = match file {
            Some(file) if uses_test_platform => command(
                "test",
                None,
                &["--project", file, "--no-build", "--no-restore"],
            ),
            _ => command("test", file, &["--no-build", "--no-restore"]),
        };

        pin_configuration(&mut test_command, configuration);

        tasks.insert(
            Id::raw("test"),
            PartialTaskConfig {
                command: Some(test_command),
                deps: Some(vec![parse_optional_target("~:build")?]),
                description: Some("Runs tests against the built assemblies. (inferred)".into()),
                inputs: Some(hash_inputs.clone()),
                ..Default::default()
            },
        );
    }

    if is_exe && wants("run") {
        tasks.insert(
            Id::raw("run"),
            PartialTaskConfig {
                command: Some(if let Some(file) = file {
                    command("run", None, &["--project", file])
                } else {
                    command("run", None, &[])
                }),
                description: Some("Runs the application locally. (inferred)".into()),
                options: Some(PartialTaskOptionsConfig {
                    cache: Some(TaskOptionCache::Enabled(false)),
                    run_in_ci: Some(TaskOptionRunInCI::Enabled(false)),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
    }

    if is_exe && is_single_tfm && wants("publish") {
        let mut publish_command = command("publish", file, &["--no-build", "--no-restore"]);
        pin_configuration(&mut publish_command, configuration);

        let mut task = PartialTaskConfig {
            command: Some(publish_command),
            deps: Some(vec![parse_optional_target("~:build")?]),
            description: Some("Publishes the built application. (inferred)".into()),
            inputs: Some(hash_inputs),
            ..Default::default()
        };

        apply_outputs(
            &mut task,
            resolve_output_path(
                evaluation.property("PublishDir"),
                inputs.project_dir,
                inputs.workspace_dir,
            ),
        )?;

        tasks.insert(Id::raw("publish"), task);
    }

    Ok(tasks)
}
