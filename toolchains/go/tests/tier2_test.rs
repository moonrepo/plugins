use moon_config::DependencyScope;
use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use std::collections::BTreeMap;
use std::path::PathBuf;

mod go_toolchain_tier2 {
    use super::*;

    mod extend_project_graph {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn loads_for_all_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("go").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert("a".into(), "a".into());
            input.project_sources.insert("b".into(), "b".into());
            input.project_sources.insert("c".into(), "c".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        ExtendProjectOutput {
                            alias: Some("example.com/org/a".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        "b".into(),
                        ExtendProjectOutput {
                            alias: Some("example.com/org/b".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        "c".into(),
                        ExtendProjectOutput {
                            alias: Some("example.com/org/c".into()),
                            dependencies: vec![
                                ProjectDependency {
                                    id: "example.com/org/a".into(),
                                    scope: DependencyScope::Production,
                                    via: Some("module example.com/org/a".into()),
                                },
                                ProjectDependency {
                                    id: "example.com/org/b".into(),
                                    scope: DependencyScope::Production,
                                    via: Some("module example.com/org/b".into()),
                                }
                            ],
                            ..Default::default()
                        }
                    ),
                ])
            );

            assert_eq!(
                output.input_files,
                [
                    PathBuf::from("/workspace/a/go.mod"),
                    PathBuf::from("/workspace/b/go.mod"),
                    PathBuf::from("/workspace/c/go.mod"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_projects_not_in_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("go").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert("a".into(), "a".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([(
                    "a".into(),
                    ExtendProjectOutput {
                        alias: Some("example.com/org/a".into()),
                        ..Default::default()
                    }
                ),])
            );

            assert_eq!(output.input_files, [PathBuf::from("/workspace/a/go.mod")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_projects_without_a_manifest() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("go").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert("no-mod".into(), "no-mod".into());

            let output = plugin.extend_project_graph(input).await;

            assert!(output.extended_projects.is_empty());
            assert!(output.input_files.is_empty());
        }
    }

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.go-bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".home/.go-bin")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fallsback_to_cargo_dir_when_no_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(
                output.paths,
                [std::env::home_dir().unwrap().join("go").join("bin")]
            );
        }
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.go-bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".home/.go-bin")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fallsback_to_cargo_dir_when_no_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(
                output.paths,
                [std::env::home_dir().unwrap().join("go").join("bin")]
            );
        }
    }

    mod locate_dependencies_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_nothing_found() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert!(output.root.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_without_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package/nested")),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_with_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package-with-sum/nested")),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(
                output.root.unwrap(),
                PathBuf::from("/workspace/package-with-sum")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/modules/a/nested"),
                    ),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["modules/a"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }
    }
}
