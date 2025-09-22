use moon_common::Id;
use moon_config::{
    DependencyScope, OneOrMany, OutputPath, PartialTaskArgs, PartialTaskConfig,
    PartialTaskDependency, PartialTaskOptionsConfig, TaskOptionRunInCI, TaskPreset,
};
use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use moon_target::Target;
use serde_json::json;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::path::PathBuf;

mod javascript_toolchain_tier2 {
    use super::*;

    mod extend_project_graph {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn loads_for_all_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("javascript").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("a"), "a".into());
            input.project_sources.insert(Id::raw("b"), "b".into());
            input.project_sources.insert(Id::raw("c"), "c".into());

            let mut output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([
                    (
                        Id::raw("a"),
                        ExtendProjectOutput {
                            alias: Some("a".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("b"),
                        ExtendProjectOutput {
                            alias: Some("@b/lib".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("c"),
                        ExtendProjectOutput {
                            alias: Some("@org/c".into()),
                            dependencies: vec![
                                ProjectDependency {
                                    id: Id::raw("a"),
                                    scope: DependencyScope::Production,
                                    via: Some("package a".into()),
                                },
                                ProjectDependency {
                                    id: Id::raw("b"),
                                    scope: DependencyScope::Development,
                                    via: Some("package @b/lib".into()),
                                }
                            ],
                            ..Default::default()
                        }
                    ),
                ])
            );

            output.input_files.sort();

            assert_eq!(
                output.input_files,
                [
                    PathBuf::from("/workspace/a/package.json"),
                    PathBuf::from("/workspace/b/package.json"),
                    PathBuf::from("/workspace/c/package.json"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_projects_not_in_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("javascript").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("a"), "a".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([(
                    Id::raw("a"),
                    ExtendProjectOutput {
                        alias: Some("a".into()),
                        ..Default::default()
                    }
                ),])
            );

            assert_eq!(
                output.input_files,
                [PathBuf::from("/workspace/a/package.json")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_projects_without_a_manifest() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("javascript").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert(Id::raw("no-manifest"), "no-manifest".into());

            let output = plugin.extend_project_graph(input).await;

            assert!(output.extended_projects.is_empty());
            assert!(output.input_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_infer_scripts_when_disbled() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("javascript").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("a"), "a".into());
            input.project_sources.insert(Id::raw("b"), "b".into());
            input.project_sources.insert(Id::raw("c"), "c".into());
            input.toolchain_config = json!({
                "inferTasksFromScripts": false,
            });

            let output = plugin.extend_project_graph(input).await;

            assert!(output.extended_projects.get("a").unwrap().tasks.is_empty());
            assert!(output.extended_projects.get("b").unwrap().tasks.is_empty());
            assert!(output.extended_projects.get("c").unwrap().tasks.is_empty());
        }

        #[allow(deprecated)]
        #[tokio::test(flavor = "multi_thread")]
        async fn infers_package_scripts_when_enabled() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("javascript").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("a"), "a".into());
            input.project_sources.insert(Id::raw("b"), "b".into());
            input.project_sources.insert(Id::raw("c"), "c".into());
            input.toolchain_config = json!({
                "inferTasksFromScripts": true,
                "packageManager": "npm"
            });

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects.get("a").unwrap().tasks,
                BTreeMap::from_iter([(
                    Id::raw("release"),
                    PartialTaskConfig {
                        description: Some("Inherited from `release` package.json script.".into()),
                        command: Some(PartialTaskArgs::List(vec![
                            "npm".into(),
                            "run".into(),
                            "release".into(),
                        ])),
                        toolchain: Some(OneOrMany::Many(vec![
                            Id::raw("javascript"),
                            Id::raw("npm"),
                            Id::raw("node"),
                        ])),
                        ..Default::default()
                    }
                )])
            );

            assert_eq!(
                output.extended_projects.get("b").unwrap().tasks,
                BTreeMap::from_iter([
                    (
                        Id::raw("build"),
                        PartialTaskConfig {
                            description: Some("Inherited from `build` package.json script.".into()),
                            command: Some(PartialTaskArgs::List(vec![
                                "npm".into(),
                                "run".into(),
                                "build".into(),
                            ])),
                            outputs: Some(vec![OutputPath::ProjectFile("build".into())]),
                            toolchain: Some(OneOrMany::Many(vec![
                                Id::raw("javascript"),
                                Id::raw("npm"),
                                Id::raw("node"),
                            ])),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("build-vite"),
                        PartialTaskConfig {
                            description: Some(
                                "Inherited from `build:vite` package.json script.".into()
                            ),
                            command: Some(PartialTaskArgs::List(vec![
                                "npm".into(),
                                "run".into(),
                                "build:vite".into(),
                            ])),
                            outputs: Some(vec![OutputPath::ProjectFile("out".into())]),
                            toolchain: Some(OneOrMany::Many(vec![
                                Id::raw("javascript"),
                                Id::raw("npm"),
                                Id::raw("node"),
                            ])),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("info"),
                        PartialTaskConfig {
                            description: Some("Inherited from `info` package.json script.".into()),
                            command: Some(PartialTaskArgs::List(vec![
                                "npm".into(),
                                "run".into(),
                                "info".into(),
                            ])),
                            toolchain: Some(OneOrMany::Many(vec![
                                Id::raw("javascript"),
                                Id::raw("npm"),
                                Id::raw("node"),
                            ])),
                            options: Some(PartialTaskOptionsConfig {
                                run_in_ci: Some(TaskOptionRunInCI::Enabled(false)),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }
                    )
                ])
            );

            assert_eq!(
                output.extended_projects.get("c").unwrap().tasks,
                BTreeMap::from_iter([
                    (
                        Id::raw("astro-serve"),
                        PartialTaskConfig {
                            description: Some(
                                "Inherited from `astro:serve` package.json script.".into()
                            ),
                            command: Some(PartialTaskArgs::List(vec![
                                "npm".into(),
                                "run".into(),
                                "astro:serve".into(),
                            ])),
                            local: Some(true),
                            preset: Some(TaskPreset::Server),
                            toolchain: Some(OneOrMany::Many(vec![
                                Id::raw("javascript"),
                                Id::raw("npm"),
                                Id::raw("node"),
                            ])),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("dev"),
                        PartialTaskConfig {
                            description: Some("Inherited from `dev` package.json script.".into()),
                            command: Some(PartialTaskArgs::List(vec![
                                "npm".into(),
                                "run".into(),
                                "dev".into(),
                            ])),
                            local: Some(true),
                            preset: Some(TaskPreset::Watcher),
                            toolchain: Some(OneOrMany::Many(vec![
                                Id::raw("javascript"),
                                Id::raw("npm"),
                                Id::raw("node"),
                            ])),
                            options: Some(PartialTaskOptionsConfig {
                                run_in_ci: Some(TaskOptionRunInCI::Enabled(false)),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("start-vite"),
                        PartialTaskConfig {
                            description: Some(
                                "Inherited from `start:vite` package.json script.".into()
                            ),
                            command: Some(PartialTaskArgs::List(vec![
                                "npm".into(),
                                "run".into(),
                                "start:vite".into(),
                            ])),
                            local: Some(true),
                            preset: Some(TaskPreset::Server),
                            toolchain: Some(OneOrMany::Many(vec![
                                Id::raw("javascript"),
                                Id::raw("npm"),
                                Id::raw("node"),
                            ])),
                            ..Default::default()
                        }
                    ),
                ])
            );
        }

        #[allow(deprecated)]
        #[tokio::test(flavor = "multi_thread")]
        async fn infers_deno_tasks_when_enabled() {
            use starbase_sandbox::pretty_assertions::assert_eq;

            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("javascript").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("a"), "a".into());
            input.project_sources.insert(Id::raw("b"), "b".into());
            input.project_sources.insert(Id::raw("c"), "c".into());
            input.toolchain_config = json!({
                "inferTasksFromScripts": true,
                "packageManager": "deno"
            });

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects.get("a").unwrap().tasks,
                BTreeMap::from_iter([
                    (
                        Id::raw("build"),
                        PartialTaskConfig {
                            description: Some("Inherited from `build` deno.json task.".into()),
                            command: Some(PartialTaskArgs::List(vec![
                                "deno".into(),
                                "task".into(),
                                "build".into(),
                            ])),
                            toolchain: Some(OneOrMany::Many(vec![
                                Id::raw("javascript"),
                                Id::raw("deno"),
                            ])),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("start"),
                        PartialTaskConfig {
                            description: Some("Inherited from `start` deno.json task.".into()),
                            command: Some(PartialTaskArgs::List(vec![
                                "deno".into(),
                                "task".into(),
                                "start".into(),
                            ])),
                            deps: Some(vec![PartialTaskDependency::Target(
                                Target::parse("~:build").unwrap()
                            )]),
                            local: Some(true),
                            preset: Some(TaskPreset::Server),
                            toolchain: Some(OneOrMany::Many(vec![
                                Id::raw("javascript"),
                                Id::raw("deno"),
                            ])),
                            ..Default::default()
                        }
                    )
                ])
            );
        }
    }

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_node_module_bins_for_each_parent_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    project: ProjectFragment {
                        source: "some/nested/path".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                [
                    sandbox.path().join("some/nested/path/node_modules/.bin"),
                    sandbox.path().join("some/nested/node_modules/.bin"),
                    sandbox.path().join("some/node_modules/.bin"),
                    sandbox.path().join("node_modules/.bin")
                ]
            );
        }
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_node_module_bins_for_each_parent_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    project: ProjectFragment {
                        source: "some/nested/path".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                [
                    sandbox.path().join("some/nested/path/node_modules/.bin"),
                    sandbox.path().join("some/nested/node_modules/.bin"),
                    sandbox.path().join("some/node_modules/.bin"),
                    sandbox.path().join("node_modules/.bin")
                ]
            );
        }
    }

    mod locate_dependencies_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_nothing_found() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert!(output.root.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_pm_not_configured() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package")),
                    toolchain_config: json!({
                        "packageManager": null
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert!(output.root.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_without_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package/nested")),
                    toolchain_config: json!({
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_with_bun_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("package/bun.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package/nested")),
                    toolchain_config: json!({
                        "packageManager": "bun"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_with_deno_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("package/deno.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package/nested")),
                    toolchain_config: json!({
                        "packageManager": "deno"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_with_npm_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("package/package-lock.json", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package/nested")),
                    toolchain_config: json!({
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_with_pnpm_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("package/pnpm-lock.yaml", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package/nested")),
                    toolchain_config: json!({
                        "packageManager": "pnpm"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_with_yarn_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("package/yarn.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package/nested")),
                    toolchain_config: json!({
                        "packageManager": "yarn"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_without_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["packages/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_bun_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("workspace/bun.lockb", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "bun"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["packages/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_deno_json() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("workspace/deno.json", r#"{ "workspace": ["packages/*"] }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "deno"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["packages/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_deno_jsonc() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file(
                "workspace/deno.jsonc",
                r#"{
    "workspace": {
        # Test nested object!
        "members": ["packages/*"],
    },
}"#,
            );

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "deno"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["packages/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_deno_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("workspace/deno.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "deno"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["packages/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_npm_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("workspace/npm-shrinkwrap.json", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["packages/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_pnpm() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("workspace/pnpm-workspace.yaml", "packages: ['apps/*']");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "pnpm"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["apps/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_pnpm_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("workspace/pnpm-lock.yaml", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "pnpm"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["packages/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_yarn_lock() {
            let sandbox = create_moon_sandbox("locate");
            sandbox.create_file("workspace/yarn.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/packages/a/nested"),
                    ),
                    toolchain_config: json!({
                        "packageManager": "yarn"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["packages/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }
    }

    mod install_dependencies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_no_pm() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "packageManager": null
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.install_command.is_none());
            assert!(output.dedupe_command.is_none());
        }

        mod bun {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "bun",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("bun", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "bun",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "bun",
                            [
                                "install",
                                "--production",
                                "--filter",
                                "foo",
                                "--filter",
                                "@scope/bar"
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_bun_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "bun",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("bun", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }
        }

        mod deno {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "deno",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("deno", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "deno",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                // Does not support focusing!
                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("deno", ["install",])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_deno_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "deno",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("deno", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }
        }

        mod npm {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "npm",
                            [
                                "install",
                                "--production",
                                "--workspace",
                                "foo",
                                "--workspace",
                                "@scope/bar"
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_npm_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn switches_to_ci_in_ci() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox
                    .create_toolchain_with_config("javascript", |cfg| {
                        cfg.host_environment(HostEnvironment {
                            ci: true,
                            ..Default::default()
                        });
                    })
                    .await;

                // Doesn't work without the lockfile
                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );

                // Now works!
                sandbox.create_file("package-lock.json", "{}");

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["ci"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }
        }

        mod pnpm {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("pnpm", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "pnpm",
                            [
                                "install",
                                "--prod",
                                "--filter-prod",
                                "foo...",
                                "--filter-prod",
                                "@scope/bar..."
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_pnpm_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("pnpm", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn legacy_dedupe_for_older_versions() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "npx",
                            [
                                "--quiet",
                                "--package",
                                "pnpm-deduplicate",
                                "--",
                                "pnpm-deduplicate"
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn modern_dedupe_for_newer_versions() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_, _| json!({ "version": "8" }));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true,
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("pnpm", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }
        }

        mod yarn {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["install", "--production"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands_for_berry() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_, _| json!({ "version": "2" }));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "yarn",
                            ["workspaces", "focus", "foo", "@scope/bar", "--production"]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_bun_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn legacy_dedupe_for_older_versions() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "npx",
                            [
                                "--quiet",
                                "--package",
                                "pnpm-deduplicate",
                                "--",
                                "pnpm-deduplicate"
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn modern_dedupe_for_newer_versions() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_, _| json!({ "version": "2" }));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true,
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }
        }
    }

    mod parse_manifest {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_workspace() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::Real(sandbox.path().join("package.json")),
                    ..Default::default()
                })
                .await;

            assert!(output.version.is_none());
            assert!(output.dev_dependencies.is_empty());
            assert!(output.build_dependencies.is_empty());
            assert!(output.peer_dependencies.is_empty());
            assert!(!output.publishable);

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        ManifestDependency::Version(UnresolvedVersionSpec::parse("1.2.3").unwrap())
                    ),
                    (
                        "b".into(),
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("^4.5.6").unwrap()
                        ),
                    ),
                    (
                        "c".into(),
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("~7.8.9").unwrap()
                        ),
                    )
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::Real(sandbox.path().join("package/package.json")),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.version.unwrap(), Version::parse("1.0.0").unwrap());
            assert!(output.publishable);

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        ManifestDependency::Version(UnresolvedVersionSpec::parse("1.2.3").unwrap())
                    ),
                    (
                        "e".into(),
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("=7.8.9").unwrap()
                        ),
                    )
                ])
            );

            assert_eq!(
                output.dev_dependencies,
                BTreeMap::from_iter([
                    (
                        "b".into(),
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("^4.5.6").unwrap()
                        ),
                    ),
                    ("f".into(), ManifestDependency::path("../other".into()))
                ])
            );

            assert_eq!(
                output.build_dependencies,
                BTreeMap::from_iter([(
                    "c".into(),
                    ManifestDependency::Version(UnresolvedVersionSpec::parse("*").unwrap())
                )])
            );

            assert_eq!(
                output.peer_dependencies,
                BTreeMap::from_iter([(
                    "d".into(),
                    ManifestDependency::Version(UnresolvedVersionSpec::parse(">=1").unwrap())
                )])
            );
        }
    }

    mod parse_lock {
        use super::*;
        use moon_pdk_test_utils::MoonWasmSandbox;

        fn create_lockfile_sandbox(pm: &str) -> MoonWasmSandbox {
            let sandbox = create_moon_sandbox("deps");
            let lockfiles = create_moon_sandbox("lockfiles");

            fs::copy_dir_all(
                lockfiles.path().join(pm),
                lockfiles.path().join(pm),
                sandbox.path(),
            )
            .unwrap();

            sandbox.debug_files();

            sandbox
        }

        fn expected_packages() -> BTreeMap<String, Option<Version>> {
            BTreeMap::from_iter([
                ("a".into(), Some(Version::new(1, 0, 0))),
                ("b".into(), Some(Version::new(2, 0, 0))),
                ("c".into(), Some(Version::new(3, 0, 0))),
            ])
        }

        fn expected_base_dependencies() -> BTreeMap<String, Vec<LockDependency>> {
            BTreeMap::from_iter([
                (
                    "csstype".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-M1uQkMl8rQK/szD0LNhtqxIPLpimGm8sOBwU7lLnCpSbTyY3yeU1Vc7l4KT5zT4s/yOxHH5O7tIuuLOCnLADRw=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("3.1.3").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "react".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-w8nqGImo45dmMIfljjMwOGtbmC/mk4CMYhWIicdSflH91J9TyCyczcPFXJzrZ/ZXcgGRFeP6BU0BEJTw6tZdfQ=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("19.1.1").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "seroval".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-RbcPH1n5cfwKrru7v7+zrZvjLurgHhGyso3HTyGtRivGWgYjbOmGuivCQaORNELjNONoK35nj28EoWul9sb1zQ=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("1.3.2").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "seroval-plugins".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-0QvCV2lM3aj/U3YozDiVwx9zpH0q8A60CTWIv4Jszj/givcudPb48B+rkU5D51NJ0pTpweGMttHjboPa9/zoIQ=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("1.3.2").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "solid-js".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-A0ZBPJQldAeGCTW0YRYJmt7RCeh5rbFfPZ2aOttgYnctHE7HgKeHCBB/PVc2P7eOfmNXqMFFFoYYdm3S4dcbkA=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("1.9.9").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "typescript".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-CWBzXQrc/qOkhidw1OzBTQuYRbfyxDXJMVJ1XNwUHGROVmuaeiEm3OslpZ1RV96d7SKKjZKrSJu3+t/xlw3R9A=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("5.9.2").unwrap()),
                        ..Default::default()
                    }]
                ),
            ])
        }

        fn expected_dependencies() -> BTreeMap<String, Vec<LockDependency>> {
            let mut map = BTreeMap::from_iter([
                (
                    "a".into(),
                    vec![LockDependency {
                        version: Some(VersionSpec::parse("1.0.0").unwrap()),
                        ..Default::default()
                    }],
                ),
                (
                    "b".into(),
                    vec![LockDependency {
                        version: Some(VersionSpec::parse("2.0.0").unwrap()),
                        ..Default::default()
                    }],
                ),
                (
                    "c".into(),
                    vec![LockDependency {
                        version: Some(VersionSpec::parse("3.0.0").unwrap()),
                        ..Default::default()
                    }],
                ),
            ]);
            map.extend(expected_base_dependencies());
            map
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_bun() {
            let sandbox = create_lockfile_sandbox("bun");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("bun.lock")),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.packages, expected_packages());
            assert_eq!(output.dependencies, expected_dependencies());
        }

        // #[tokio::test(flavor = "multi_thread")]
        // async fn parses_bun_classic() {
        //     let sandbox = create_lockfile_sandbox("bun-classic");
        //     let plugin = sandbox.create_toolchain("javascript").await;

        //     let output = plugin
        //         .parse_lock(ParseLockInput {
        //             path: VirtualPath::Real(sandbox.path().join("bun.lockb")),
        //             ..Default::default()
        //         })
        //         .await;

        //     // Workspaces packages have `workspace:` in their version
        //     assert_eq!(
        //         output.packages,
        //         BTreeMap::from_iter([("a".into(), None), ("b".into(), None), ("c".into(), None)])
        //     );

        //     assert_eq!(output.dependencies, {
        //         let mut deps = BTreeMap::from_iter([
        //             ("a".into(), vec![LockDependency::default()]),
        //             ("b".into(), vec![LockDependency::default()]),
        //             ("c".into(), vec![LockDependency::default()]),
        //         ]);
        //         deps.extend(expected_base_dependencies());
        //         deps
        //     });
        // }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_deno() {
            let sandbox = create_lockfile_sandbox("deno");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("deno.lock")),
                    ..Default::default()
                })
                .await;

            assert!(output.packages.is_empty());
            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "jsr:@astral/astral".into(),
                        vec![LockDependency {
                            hash: Some(
                                "d6a4628313d8be99aac0f51005c1dc090fa3b4c6b5c8335c26a52d4842aa1276"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("0.5.3").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "jsr:@zip-js/zip-js".into(),
                        vec![LockDependency {
                            hash: Some(
                                "14c123f0e534377a6f47c5ba5293bb6c0f3e72e78c6a687108011605420a4867"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("2.7.73").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "npm:@babel/core".into(),
                        vec![LockDependency {
                            hash: Some(
                                "sha512-yDBHV9kQNcr2/sUr9jghVyz9C3Y5G2zUM2H2lo+9mKv4sFgbA8s8Z9t8D1jiTkGoO/NoIfKMyKWr4s6CN23ZwQ=="
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("7.28.3").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "npm:@babel/preset-react".into(),
                        vec![LockDependency {
                            hash: Some(
                                "sha512-oJHWh2gLhU9dW9HHr42q0cI0/iHHXTLGe39qvpAZZzagHy0MzYLCnCVV0symeRvzmjHyVU7mw2K06E6u/JwbhA=="
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("7.27.1").unwrap()),
                            ..Default::default()
                        }]
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_npm() {
            let sandbox = create_lockfile_sandbox("npm");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("package-lock.json")),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.packages, expected_packages());
            assert_eq!(output.dependencies, expected_dependencies());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_pnpm() {
            let sandbox = create_lockfile_sandbox("pnpm");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("pnpm-lock.yaml")),
                    ..Default::default()
                })
                .await;

            assert!(output.packages.is_empty());
            assert_eq!(output.dependencies, expected_base_dependencies());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_yarn() {
            let sandbox = create_lockfile_sandbox("yarn");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("yarn.lock")),
                    ..Default::default()
                })
                .await;

            // Workspace packages in the lockfile have their version
            // set to `0.0.0-use.local` instead of the actual version
            assert_eq!(
                output.packages,
                BTreeMap::from_iter([
                    ("a".into(), Version::parse("0.0.0-use.local").ok()),
                    ("b".into(), Version::parse("0.0.0-use.local").ok()),
                    ("c".into(), Version::parse("0.0.0-use.local").ok()),
                ])
            );

            // Yarn has different integrities than other package managers...
            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        vec![LockDependency {
                            version: VersionSpec::parse("0.0.0-use.local").ok(),
                            ..Default::default()
                        }],
                    ),
                    (
                        "b".into(),
                        vec![LockDependency {
                            version: VersionSpec::parse("0.0.0-use.local").ok(),
                            ..Default::default()
                        }],
                    ),
                    (
                        "c".into(),
                        vec![LockDependency {
                            version: VersionSpec::parse("0.0.0-use.local").ok(),
                            ..Default::default()
                        }],
                    ),
                    (
                        "csstype".into(),
                        vec![LockDependency {
                            hash: Some(
                                "10c0/80c089d6f7e0c5b2bd83cf0539ab41474198579584fa10d86d0cafe0642202343cbc119e076a0b1aece191989477081415d66c9fefbf3c957fc2fc4b7009f248"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("3.1.3").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "react".into(),
                        vec![LockDependency {
                            hash: Some(
                                "10c0/8c9769a2dfd02e603af6445058325e6c8a24b47b185d0e461f66a6454765ddcaecb3f0a90184836c68bb509f3c38248359edbc42f0d07c23eb500a5c30c87b4e"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("19.1.1").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "seroval".into(),
                        vec![LockDependency {
                            hash: Some(
                                "10c0/19e74825643786d22e5c58054bd28065238de0156545afba82f9a7d3ee70ea4f0249b427f317bc6bf983849dde8e4190264728d90c84620aa163bfbc5971f1bc"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("1.3.2").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "seroval-plugins".into(),
                        vec![LockDependency {
                            hash: Some(
                                "10c0/67b108b3cbc189acca445b512ebd77e11b55c6aa3d1610c3a0b4822b63e5c6d0a4426ac6e50574772cc743257f0a16a8a4d12e5e4f28a2da8e1f583b00a27bbe"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("1.3.2").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "solid-js".into(),
                        vec![LockDependency {
                            hash: Some(
                                "10c0/bd3fda22b66c4955bb4f29236b622dcfb11b7f37ec7fa974f366292bef1076750fade2e656d584cd977f50354b23cfca4026e8ae5153e991c8c112b15b21c9e3"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("1.9.9").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "typescript".into(),
                        vec![LockDependency {
                            hash: Some(
                                "10c0/cd635d50f02d6cf98ed42de2f76289701c1ec587a363369255f01ed15aaf22be0813226bff3c53e99d971f9b540e0b3cc7583dbe05faded49b1b0bed2f638a18"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("5.9.2").unwrap()),
                            ..Default::default()
                        }]
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_yarn_classic() {
            let sandbox = create_lockfile_sandbox("yarn-classic");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("yarn.lock")),
                    ..Default::default()
                })
                .await;

            assert!(output.packages.is_empty());
            assert_eq!(output.dependencies, expected_base_dependencies());
        }
    }
}
