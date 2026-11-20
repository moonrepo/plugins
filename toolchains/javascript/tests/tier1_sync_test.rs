use javascript_toolchain::config::JavaScriptDependencyVersionFormat;
use moon_config::DependencyScope;
use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use starbase_sandbox::assert_snapshot;

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
                        id: Id::raw("package"),
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
                        id: Id::raw("root"),
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
                        id: Id::raw("package"),
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
                        id: Id::raw("package-clean"),
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

        fn create_root_only_input(
            deps: serde_json::Value,
        ) -> (moon_pdk_test_utils::MoonWasmSandbox, SyncProjectInput) {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "app/package.json",
                json!({ "name": "app", "dependencies": deps, "devDependencies": {} }).to_string(),
            );
            sandbox.create_file(
                "lib/package.json",
                r#"{ "name": "lib", "version": "1.0.0" }"#,
            );

            let input = SyncProjectInput {
                project: ProjectFragment {
                    id: Id::raw("app"),
                    source: "app".into(),
                    toolchains: vec![Id::raw("javascript")],
                    ..Default::default()
                },
                project_dependencies: vec![ProjectFragment {
                    id: Id::raw("lib"),
                    source: "lib".into(),
                    dependency_scope: Some(DependencyScope::Production),
                    toolchains: vec![Id::raw("javascript")],
                    ..Default::default()
                }],
                toolchain_config: json!({
                    "rootPackageDependenciesOnly": true,
                    "syncProjectWorkspaceDependencies": true,
                    "packageManager": "pnpm"
                }),
                ..Default::default()
            };

            (sandbox, input)
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn allows_local_workspace_dependencies() {
            let (sandbox, input) = create_root_only_input(json!({ "lib": "workspace:*" }));
            let plugin = sandbox.create_toolchain("javascript").await;

            plugin.sync_project(input).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn allows_what_the_sync_writes() {
            let (sandbox, input) = create_root_only_input(json!({}));
            let plugin = sandbox.create_toolchain("javascript").await;

            plugin.sync_project(input.clone()).await;

            assert!(
                std::fs::read_to_string(sandbox.path().join("app/package.json"))
                    .unwrap()
                    .contains(r#""lib": "workspace:*""#)
            );

            // Second run reads what the first one wrote
            plugin.sync_project(input).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Dependencies can only be defined in the root package.json")]
        async fn errors_for_external_dependencies_next_to_local() {
            let (sandbox, input) =
                create_root_only_input(json!({ "lib": "workspace:*", "react": "^19.0.0" }));
            let plugin = sandbox.create_toolchain("javascript").await;

            plugin.sync_project(input).await;
        }
    }

    mod sync_project_workspace_dependencies {
        use super::*;

        async fn test_version_format(format: JavaScriptDependencyVersionFormat) {
            let sandbox = create_moon_sandbox("deps-sync");
            // Berry, as Yarn 1 has no `workspace:` protocol
            sandbox.create_file("yarn.lock", "__metadata:\n  version: 8\n");

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
                vec![VirtualPath::new("/workspace/base/package.json")]
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

        fn create_dep(id: &str, scope: DependencyScope) -> ProjectFragment {
            ProjectFragment {
                id: Id::raw(id),
                source: id.into(),
                dependency_scope: Some(scope),
                toolchains: vec![Id::raw("javascript")],
                ..Default::default()
            }
        }

        fn create_input(
            deps: Vec<ProjectFragment>,
            format: JavaScriptDependencyVersionFormat,
        ) -> SyncProjectInput {
            SyncProjectInput {
                project: ProjectFragment {
                    id: Id::raw("app"),
                    source: "app".into(),
                    toolchains: vec![Id::raw("javascript")],
                    ..Default::default()
                },
                project_dependencies: deps,
                toolchain_config: json!({
                    "dependencyVersionFormat": format.to_string(),
                    "syncProjectWorkspaceDependencies": true,
                    "packageManager": "npm"
                }),
                ..Default::default()
            }
        }

        fn read_app(sandbox: &moon_pdk_test_utils::MoonWasmSandbox) -> serde_json::Value {
            serde_json::from_str(
                &std::fs::read_to_string(sandbox.path().join("app/package.json")).unwrap(),
            )
            .unwrap()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_projects_without_a_manifest() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "lib/package.json",
                r#"{ "name": "lib", "version": "1.0.0" }"#,
            );

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .sync_project(create_input(
                    vec![create_dep("lib", DependencyScope::Production)],
                    JavaScriptDependencyVersionFormat::Workspace,
                ))
                .await;

            assert!(output.skipped);
            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_the_minor_version_for_0x_peers() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/package.json", r#"{ "name": "app" }"#);
            sandbox.create_file(
                "zero/package.json",
                r#"{ "name": "zero", "version": "0.3.0" }"#,
            );
            sandbox.create_file(
                "one/package.json",
                r#"{ "name": "one", "version": "1.2.0" }"#,
            );

            let plugin = sandbox.create_toolchain("javascript").await;

            plugin
                .sync_project(create_input(
                    vec![
                        create_dep("zero", DependencyScope::Peer),
                        create_dep("one", DependencyScope::Peer),
                    ],
                    JavaScriptDependencyVersionFormat::Workspace,
                ))
                .await;

            let app = read_app(&sandbox);

            assert_eq!(app["peerDependencies"]["zero"], "^0.3.0");
            assert_eq!(app["peerDependencies"]["one"], "^1.0.0");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn syncs_packages_without_a_version() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/package.json", r#"{ "name": "app" }"#);
            sandbox.create_file("nov/package.json", r#"{ "name": "nov" }"#);
            sandbox.create_file("peer/package.json", r#"{ "name": "peer" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            plugin
                .sync_project(create_input(
                    vec![
                        create_dep("nov", DependencyScope::Production),
                        create_dep("peer", DependencyScope::Peer),
                    ],
                    JavaScriptDependencyVersionFormat::Star,
                ))
                .await;

            let app = read_app(&sandbox);

            assert_eq!(app["dependencies"]["nov"], "*");
            assert_eq!(app["peerDependencies"]["peer"], "*");
        }

        // nub writes `workspace:` into `package-lock.json`, which npm can't read (NUB.md D2)
        #[tokio::test(flavor = "multi_thread")]
        async fn syncs_npm_format_for_nub_in_npm_repos() {
            for (lock_name, expected) in [
                ("package-lock.json", "file:../lib"),
                ("nub.lock", "workspace:*"),
            ] {
                let sandbox = create_empty_moon_sandbox();
                sandbox.create_file(lock_name, "{}");
                sandbox.create_file("app/package.json", r#"{ "name": "app" }"#);
                sandbox.create_file(
                    "lib/package.json",
                    r#"{ "name": "lib", "version": "1.0.0" }"#,
                );

                let plugin = sandbox.create_toolchain("javascript").await;
                let mut input = create_input(
                    vec![create_dep("lib", DependencyScope::Production)],
                    JavaScriptDependencyVersionFormat::Workspace,
                );
                input.toolchain_config["packageManager"] = json!("nub");

                plugin.sync_project(input).await;

                assert_eq!(
                    read_app(&sandbox)["dependencies"]["lib"],
                    expected,
                    "{lock_name}"
                );
            }
        }

        // Yarn 1 has no `workspace:` protocol (YARN.md #5)
        #[tokio::test(flavor = "multi_thread")]
        async fn syncs_versions_for_yarn_1() {
            for (version, lockfile, format, expected) in [
                (
                    None,
                    None,
                    JavaScriptDependencyVersionFormat::Workspace,
                    "*",
                ),
                (
                    None,
                    Some("# yarn lockfile v1\n"),
                    JavaScriptDependencyVersionFormat::WorkspaceCaret,
                    "^1.0.0",
                ),
                (
                    Some("1.22.22"),
                    None,
                    JavaScriptDependencyVersionFormat::WorkspaceTilde,
                    "~1.0.0",
                ),
                (
                    Some("1.22.22"),
                    None,
                    JavaScriptDependencyVersionFormat::Link,
                    "link:../lib",
                ),
                // Berry keeps them
                (
                    None,
                    Some("__metadata:\n  version: 8\n"),
                    JavaScriptDependencyVersionFormat::Workspace,
                    "workspace:*",
                ),
                (
                    Some("4.18.1"),
                    None,
                    JavaScriptDependencyVersionFormat::WorkspaceCaret,
                    "workspace:^",
                ),
            ] {
                let mut sandbox = create_empty_moon_sandbox();
                sandbox.create_file("app/package.json", r#"{ "name": "app" }"#);
                sandbox.create_file(
                    "lib/package.json",
                    r#"{ "name": "lib", "version": "1.0.0" }"#,
                );

                if let Some(content) = lockfile {
                    sandbox.create_file("yarn.lock", content);
                }

                sandbox.host_funcs.mock_load_toolchain_config(move |id, _| {
                    assert_eq!(id, "yarn");
                    json!({ "version": version })
                });

                let plugin = sandbox.create_toolchain("javascript").await;
                let mut input =
                    create_input(vec![create_dep("lib", DependencyScope::Production)], format);
                input.toolchain_config["packageManager"] = json!("yarn");

                plugin.sync_project(input).await;

                assert_eq!(
                    read_app(&sandbox)["dependencies"]["lib"],
                    expected,
                    "{version:?} {lockfile:?} {format:?}"
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_packages_without_a_version_for_version_formats() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/package.json", r#"{ "name": "app" }"#);
            sandbox.create_file("nov/package.json", r#"{ "name": "nov" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .sync_project(create_input(
                    vec![create_dep("nov", DependencyScope::Production)],
                    JavaScriptDependencyVersionFormat::VersionCaret,
                ))
                .await;

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replaces_stale_version_ranges() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "app/package.json",
                json!({
                    "name": "app",
                    "dependencies": { "stale": "^0.3.0", "fresh": "^1.0.0", "ws": "workspace:*" },
                    "peerDependencies": { "peer": "^1.0.0" }
                })
                .to_string(),
            );
            sandbox.create_file(
                "stale/package.json",
                r#"{ "name": "stale", "version": "0.4.0" }"#,
            );
            sandbox.create_file(
                "fresh/package.json",
                r#"{ "name": "fresh", "version": "1.5.0" }"#,
            );
            sandbox.create_file("ws/package.json", r#"{ "name": "ws", "version": "2.0.0" }"#);
            sandbox.create_file(
                "peer/package.json",
                r#"{ "name": "peer", "version": "2.0.0" }"#,
            );

            let plugin = sandbox.create_toolchain("javascript").await;

            plugin
                .sync_project(create_input(
                    vec![
                        create_dep("stale", DependencyScope::Production),
                        create_dep("fresh", DependencyScope::Production),
                        create_dep("ws", DependencyScope::Production),
                        create_dep("peer", DependencyScope::Peer),
                    ],
                    JavaScriptDependencyVersionFormat::VersionCaret,
                ))
                .await;

            let app = read_app(&sandbox);

            assert_eq!(app["dependencies"]["stale"], "^0.4.0");
            assert_eq!(app["dependencies"]["fresh"], "^1.0.0");
            assert_eq!(app["dependencies"]["ws"], "workspace:*");
            assert_eq!(app["peerDependencies"]["peer"], "^2.0.0");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_the_text_of_entries_it_doesnt_change() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "app/package.json",
                json!({
                    "name": "app",
                    "dependencies": { "range": ">=1.0.0 <2.0.0", "partial": "18.x" }
                })
                .to_string(),
            );
            sandbox.create_file(
                "lib/package.json",
                r#"{ "name": "lib", "version": "1.0.0" }"#,
            );

            let plugin = sandbox.create_toolchain("javascript").await;

            plugin
                .sync_project(create_input(
                    vec![create_dep("lib", DependencyScope::Production)],
                    JavaScriptDependencyVersionFormat::File,
                ))
                .await;

            let app = read_app(&sandbox);

            assert_eq!(app["dependencies"]["lib"], "file:../lib");
            assert_eq!(app["dependencies"]["range"], ">=1.0.0 <2.0.0");
            assert_eq!(app["dependencies"]["partial"], "18.x");
        }
    }
}
