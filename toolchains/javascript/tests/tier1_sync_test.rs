use javascript_toolchain::JavaScriptDependencyVersionFormat;
use moon_config::DependencyScope;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_moon_sandbox;
use serde_json::json;
use starbase_sandbox::assert_snapshot;
use std::path::PathBuf;

mod javascript_toolchain_tier1 {
    use super::*;

    mod root_package_dependencies_only {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_not_enabled() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: ProjectFragment {
                        source: "package".into(),
                        toolchains: vec![Id::raw("javascript")],
                        ..Default::default()
                    },
                    toolchain_config: json!({
                        "rootPackageDependenciesOnly": false
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_for_root_project() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: ProjectFragment {
                        source: ".".into(),
                        toolchains: vec![Id::raw("javascript")],
                        ..Default::default()
                    },
                    toolchain_config: json!({
                        "rootPackageDependenciesOnly": true
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Dependencies can only be defined in the root package.json")]
        async fn errors_for_nested_project() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("javascript").await;

            plugin
                .sync_project(SyncProjectInput {
                    project: ProjectFragment {
                        source: "package".into(),
                        toolchains: vec![Id::raw("javascript")],
                        ..Default::default()
                    },
                    toolchain_config: json!({
                        "rootPackageDependenciesOnly": true
                    }),
                    ..Default::default()
                })
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_error_for_nested_project_if_no_deps() {
            let sandbox = create_moon_sandbox("files");
            sandbox.create_file("package-clean/package.json", "{}");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: ProjectFragment {
                        source: "package-clean".into(),
                        toolchains: vec![Id::raw("javascript")],
                        ..Default::default()
                    },
                    toolchain_config: json!({
                        "rootPackageDependenciesOnly": true
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
        }
    }

    mod sync_project_workspace_dependencies {
        use super::*;

        async fn test_version_format(format: JavaScriptDependencyVersionFormat) {
            let mut sandbox = create_moon_sandbox("deps-sync");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "version": "1.2.3" }));

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: ProjectFragment {
                        id: Id::raw("base"),
                        source: "base".into(),
                        toolchains: vec![Id::raw("javascript")],
                        ..Default::default()
                    },
                    project_dependencies: vec![
                        ProjectFragment {
                            id: Id::raw("a"),
                            source: "a".into(),
                            dependency_scope: Some(DependencyScope::Production),
                            toolchains: vec![Id::raw("javascript")],
                            ..Default::default()
                        },
                        ProjectFragment {
                            id: Id::raw("b"),
                            source: "b".into(),
                            dependency_scope: Some(DependencyScope::Development),
                            toolchains: vec![Id::raw("javascript")],
                            ..Default::default()
                        },
                        ProjectFragment {
                            id: Id::raw("c"),
                            source: "c".into(),
                            dependency_scope: Some(DependencyScope::Peer),
                            toolchains: vec![Id::raw("javascript")],
                            ..Default::default()
                        },
                        ProjectFragment {
                            id: Id::raw("no-scope"),
                            source: "no-scope".into(),
                            toolchains: vec![Id::raw("javascript")],
                            ..Default::default()
                        },
                        ProjectFragment {
                            id: Id::raw("no-toolchain"),
                            source: "no-toolchain".into(),
                            dependency_scope: Some(DependencyScope::Peer),
                            ..Default::default()
                        },
                        ProjectFragment {
                            id: Id::raw("other-toolchain"),
                            source: "other-toolchain".into(),
                            toolchains: vec![Id::raw("typescript")],
                            ..Default::default()
                        },
                        ProjectFragment {
                            id: Id::raw("root"),
                            source: ".".into(),
                            dependency_scope: Some(DependencyScope::Root),
                            toolchains: vec![Id::raw("javascript")],
                            ..Default::default()
                        },
                    ],
                    toolchain_config: json!({
                        "dependencyVersionFormat": format.to_string(),
                        "syncProjectWorkspaceDependencies": true,
                        "packageManager": "yarn"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!output.operations.is_empty());
            assert_eq!(
                output.changed_files,
                vec![PathBuf::from("/workspace/base/package.json")]
            );
            assert_snapshot!(
                format!("format_{format}"),
                std::fs::read_to_string(sandbox.path().join("base/package.json")).unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_file_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::File).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_link_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::Link).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_star_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::Star).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_version_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::Version).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_version_caret_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::VersionCaret).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_version_tilde_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::VersionTilde).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_workspace_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::Workspace).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_workspace_caret_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::WorkspaceCaret).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn as_workspace_tilde_dependency() {
            test_version_format(JavaScriptDependencyVersionFormat::WorkspaceTilde).await;
        }
    }
}
