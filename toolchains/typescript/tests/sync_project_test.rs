mod utils;

use moon_config::DependencyScope;
use moon_pdk::SyncProjectInput;
use moon_pdk_test_utils::create_moon_sandbox;
use serde_json::json;
use starbase_sandbox::assert_snapshot;
use starbase_utils::fs;
use utils::*;

mod sync_project {
    use super::*;

    mod create_missing_config {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_if_missing() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;
            let cfg_path = sandbox.path().join("no-config/tsconfig.json");

            assert!(!cfg_path.exists());

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-config"),
                    toolchain_config: json!({
                        "createMissingConfig": true,
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(
                &output,
                "/workspace/no-config/tsconfig.json"
            ));
            assert!(cfg_path.exists());
            assert_snapshot!(fs::read_file(cfg_path).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_if_missing_with_custom_options() {
            let sandbox = create_moon_sandbox("create-custom");
            let plugin = sandbox.create_toolchain("typescript").await;
            let cfg_path = sandbox.path().join("no-config/tsconfig.ref.json");

            assert!(!cfg_path.exists());

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-config"),
                    toolchain_config: json!({
                        "createMissingConfig": true,
                        "projectConfigFileName": "tsconfig.ref.json",
                        "rootOptionsConfigFileName": "tsconfig.base.json",
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(
                &output,
                "/workspace/no-config/tsconfig.ref.json"
            ));
            assert!(cfg_path.exists());
            assert_snapshot!(fs::read_file(cfg_path).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_creates_if_disabled() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;
            let cfg_path = sandbox.path().join("no-config/tsconfig.json");

            assert!(!cfg_path.exists());

            plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-config"),
                    toolchain_config: json!({
                        "createMissingConfig": false,
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!cfg_path.exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_creates_if_not_syncing() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;
            let cfg_path = sandbox.path().join("no-config/tsconfig.json");

            assert!(!cfg_path.exists());

            plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-config"),
                    toolchain_config: json!({
                        "createMissingConfig": true,
                        "syncProjectReferences": false,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!cfg_path.exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_if_exists() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;
            let cfg_path = sandbox.path().join("no-options/tsconfig.json");

            assert!(cfg_path.exists());

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-options"),
                    toolchain_config: json!({
                        "createMissingConfig": true,
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!has_changed_file(
                &output,
                "/workspace/no-options/tsconfig.json"
            ));
            assert!(cfg_path.exists());
        }
    }

    mod sync_project_references {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_deps_as_refs() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(
                &output,
                "/workspace/no-refs/tsconfig.json"
            ));
            assert_snapshot!(fs::read_file(sandbox.path().join("no-refs/tsconfig.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_dupe_add_refs() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("some-refs"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(
                &output,
                "/workspace/some-refs/tsconfig.json"
            ));
            assert_snapshot!(
                fs::read_file(sandbox.path().join("some-refs/tsconfig.json")).unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_all_refs_exist() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("all-refs"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!has_changed_file(
                &output,
                "/workspace/all-refs/tsconfig.json"
            ));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "syncProjectReferences": false,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!has_changed_file(
                &output,
                "/workspace/no-refs/tsconfig.json"
            ));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_invalid_deps() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    project_dependencies: vec![
                        // Root
                        {
                            let mut dep = create_project("root");
                            dep.source = ".".into();
                            dep
                        },
                        // Root scope
                        {
                            let mut dep = create_project("root-scope");
                            dep.dependency_scope = Some(DependencyScope::Root);
                            dep
                        },
                        // Build scope
                        {
                            let mut dep = create_project("build-scope");
                            dep.dependency_scope = Some(DependencyScope::Build);
                            dep
                        },
                        // Not TS enabled
                        {
                            let mut dep = create_project("not-ts");
                            dep.toolchains.clear();
                            dep
                        },
                        // No tsconfig
                        create_project("d"),
                    ],
                    toolchain_config: json!({
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!has_changed_file(
                &output,
                "/workspace/no-refs/tsconfig.json"
            ));
        }
    }
}
