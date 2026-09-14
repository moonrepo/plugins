use moon_pdk_api::config_struct;
use schematic::{Config, Schematic};
use std::collections::BTreeMap;

/// The task names the plugin can infer.
pub const INFERABLE_TASKS: &[&str] = &["build", "test", "run", "publish"];

/// Which tasks to infer from evaluated MSBuild properties: a boolean to
/// enable/disable all of them, or an explicit list of task names
/// (`build`, `test`, `run`, `publish`) to infer only those.
#[derive(Clone, Debug, PartialEq, Schematic, serde::Deserialize, serde::Serialize)]
#[serde(
    untagged,
    expecting = "expected a boolean or a list of task names (build, test, run, publish)"
)]
pub enum InferTasksSetting {
    Enabled(bool),
    Only(Vec<String>),
}

impl Default for InferTasksSetting {
    fn default() -> Self {
        Self::Enabled(true)
    }
}

impl InferTasksSetting {
    /// Is a specific task name selected for inference?
    pub fn includes(&self, task: &str) -> bool {
        match self {
            Self::Enabled(enabled) => *enabled,
            Self::Only(list) => list.iter().any(|name| name.eq_ignore_ascii_case(task)),
        }
    }

    /// Is any inference enabled at all?
    pub fn any_enabled(&self) -> bool {
        match self {
            Self::Enabled(enabled) => *enabled,
            Self::Only(list) => !list.is_empty(),
        }
    }
}

config_struct!(
    /// Configures and enables the .NET toolchain.
    #[derive(Config)]
    pub struct DotnetToolchainConfig {
        /// Infer moon project dependencies from MSBuild `ProjectReference`
        /// items, so `moon` knows the real build order without any `dependsOn`
        /// declarations.
        ///
        /// This runs a real MSBuild evaluation, which is what makes it see
        /// references added by `Directory.Build.targets` and conditional
        /// `ProjectReference`s, not just what a project file lists literally.
        /// Every project in the workspace is evaluated in a single batched
        /// invocation. A reference outside the moon workspace is skipped.
        ///
        /// Defaults to `true`.
        #[setting(default = true)]
        pub infer_dependencies: bool,

        /// Infer `build`, `test`, `run` and `publish` tasks from each project's
        /// evaluated MSBuild properties.
        ///
        /// `true` infers all four, `false` infers none, and a list such as
        /// `['build', 'test']` infers only the named ones. Unrecognised names
        /// are ignored rather than rejected. The setting is workspace-level, so
        /// turning inference off never requires per-project overrides.
        ///
        /// What gets inferred:
        ///
        /// - `build` for every project, with `deps: ['^:build']` and
        ///   `--no-dependencies`, so moon orchestrates and caches the graph
        ///   per project rather than delegating that to MSBuild.
        /// - `test` for a project with `IsTestProject=true` or a
        ///   `Microsoft.NET.Test.Sdk` reference. Both VSTest and
        ///   Microsoft.Testing.Platform are supported; the command shape follows
        ///   whichever the governing `global.json` selects.
        /// - `run` for `Exe`/`WinExe`, never cached and excluded from CI.
        /// - `publish` for single-target-framework `Exe`/`WinExe`. Multi-TFM
        ///   projects get none, since `dotnet publish` needs an explicit `-f`.
        ///
        /// Never inferred: `pack`, `watch`, `clean`, and `restore`. moon models
        /// restore as the install-dependencies action instead, which is why the
        /// inferred commands all pass `--no-restore`.
        ///
        /// Your own tasks always win. A task of the same id in a project's
        /// `moon.yml` replaces the inferred one, and an id defined by an
        /// inherited task file (`.moon/tasks.yml`, `.moon/tasks/**/*.yml`) that
        /// can apply to dotnet projects is not inferred at all, since moon would
        /// merge the two into a broken command. Each suppression is logged with
        /// the id and the file that claimed it.
        ///
        /// Inferred commands pin the evaluated `Configuration` with `-c`, since
        /// `dotnet publish` defaults to Release on .NET 8+ while `build`
        /// defaults to Debug and `--no-build` needs them to agree. Task outputs
        /// come from the evaluated `BaseOutputPath`/`PublishDir`. A path
        /// resolving outside the workspace makes the task run uncached rather
        /// than cache the wrong directory.
        ///
        /// Defaults to `true`.
        pub infer_tasks: InferTasksSetting,

        /// Additional arguments appended to `dotnet restore`, which moon runs as
        /// its install-dependencies action rather than as a task.
        ///
        /// `--locked-mode` is added automatically when a `packages.lock.json` (or
        /// a `packages.<project>.lock.json`) is found, so it does not need to be
        /// passed here.
        pub restore_args: Vec<String>,

        /// Additional MSBuild properties applied to every evaluation behind
        /// dependency and task inference, passed as `-p:NAME=VALUE`.
        ///
        /// Use this when the graph must be evaluated the way the code is
        /// actually built. The case this exists for is a conditional,
        /// codegen-only `ProjectReference` (`ReferenceOutputAssembly=false`,
        /// gated on something like
        /// `Condition="'$(SkipApiClientGen)' != 'true'"`) that a production
        /// build disables. Without the property, evaluation contributes
        /// build-ordering edges the real build never compiles.
        ///
        /// ```yaml
        /// dotnet:
        ///   msbuildProperties:
        ///     SkipApiClientGen: 'true'
        /// ```
        ///
        /// These apply to evaluation only. Inferred task commands and
        /// `dotnet restore` do not pass them, so `moon run` builds stay what
        /// the project defines. Two consequences:
        ///
        /// - Setting a property your real build does not set makes the graph
        ///   describe a build nobody runs. Inferred `build` tasks pass
        ///   `--no-dependencies`, so moon is the only thing ordering
        ///   dependencies, and a dropped edge drops that ordering.
        /// - A `PackageReference` gated on one of these is resolved for hashing
        ///   but not for restore, so the recorded package set can differ from
        ///   what restore installs.
        pub msbuild_properties: BTreeMap<String, String>,
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_schema_builds() {
        let schema = schematic::SchemaBuilder::build_root::<DotnetToolchainConfig>();
        let json = serde_json::to_string(&schema).unwrap();

        assert!(json.contains("inferDependencies"));
        assert!(json.contains("inferTasks"));
        assert!(json.contains("restoreArgs"));
        assert!(json.contains("msbuildProperties"));

        // `inferTasks` must stay a `bool | string[]` union. The derive produces
        // this from the untagged enum; asserting the shape means a change to the
        // enum cannot silently narrow what the setting accepts.
        assert!(
            json.contains(
                r#""operator":"AnyOf","variants_types":[{"ty":{"type":"Boolean"}},{"ty":{"type":"Array","items_type":{"ty":{"type":"String"}}}}]"#
            ),
            "inferTasks lost its bool | string[] union: {json}"
        );
    }

    #[test]
    fn config_defaults_apply() {
        let config: DotnetToolchainConfig = serde_json::from_value(serde_json::json!({})).unwrap();

        assert!(config.infer_dependencies);
        assert_eq!(config.infer_tasks, InferTasksSetting::Enabled(true));
        assert!(config.infer_tasks.any_enabled());
        assert!(config.restore_args.is_empty());
        assert!(config.msbuild_properties.is_empty());
    }

    #[test]
    fn msbuild_properties_deserialize_from_camel_case() {
        let config: DotnetToolchainConfig = serde_json::from_value(serde_json::json!({
            "msbuildProperties": { "SkipApiClientGen": "true", "Answer": "42" }
        }))
        .unwrap();

        assert_eq!(
            config.msbuild_properties.get("SkipApiClientGen"),
            Some(&"true".to_owned())
        );
        assert_eq!(
            config.msbuild_properties.get("Answer"),
            Some(&"42".to_owned())
        );
    }

    #[test]
    fn infer_tasks_accepts_bool_and_list() {
        let config: DotnetToolchainConfig =
            serde_json::from_value(serde_json::json!({ "inferTasks": false })).unwrap();
        assert!(!config.infer_tasks.any_enabled());
        assert!(!config.infer_tasks.includes("build"));

        let config: DotnetToolchainConfig =
            serde_json::from_value(serde_json::json!({ "inferTasks": ["build", "Test"] })).unwrap();
        assert!(config.infer_tasks.any_enabled());
        assert!(config.infer_tasks.includes("build"));
        assert!(config.infer_tasks.includes("test"));
        assert!(!config.infer_tasks.includes("run"));
        assert!(!config.infer_tasks.includes("publish"));

        let config: DotnetToolchainConfig =
            serde_json::from_value(serde_json::json!({ "inferTasks": [] })).unwrap();
        assert!(!config.infer_tasks.any_enabled());
    }
}
