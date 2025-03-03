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

    mod sync_root_project_reference {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_project_as_ref() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    toolchain_config: json!({
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(&output, "/workspace/./tsconfig.json"));
            assert_snapshot!(fs::read_file(sandbox.path().join("tsconfig.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_project_as_ref_with_custom_options() {
            let sandbox = create_moon_sandbox("refs-custom");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    toolchain_config: json!({
                        "projectConfigFileName": "tsconfig.ref.json",
                        "rootConfigFileName": "tsconfig.root.json",
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(&output, "/workspace/./tsconfig.root.json"));
            assert_snapshot!(fs::read_file(sandbox.path().join("tsconfig.root.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_if_disabled() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    toolchain_config: json!({
                        "syncProjectReferences": false,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!has_changed_file(&output, "/workspace/./tsconfig.json"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_root_level_project_if_self() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let mut project = create_project("root");
            project.source = ".".into();

            let output = plugin
                .sync_project(SyncProjectInput {
                    project,
                    toolchain_config: json!({
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!has_changed_file(&output, "/workspace/./tsconfig.json"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn works_with_root_level_project() {
            let sandbox = create_moon_sandbox("refs-root-level");
            let plugin = sandbox.create_toolchain("typescript").await;

            let mut project = create_project("root");
            project.source = ".".into();

            let output = plugin
                .sync_project(SyncProjectInput {
                    project,
                    toolchain_config: json!({
                        "projectConfigFileName": "tsconfig.project.json",
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(&output, "/workspace/./tsconfig.json"));
            assert_snapshot!(fs::read_file(sandbox.path().join("tsconfig.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn works_with_root_dir() {
            let sandbox = create_moon_sandbox("refs-sibling-root");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    toolchain_config: json!({
                        "root": "root",
                        "syncProjectReferences": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(&output, "/workspace/root/tsconfig.json"));
            assert_snapshot!(fs::read_file(sandbox.path().join("root/tsconfig.json")).unwrap());
        }
    }

    mod include_shared_types {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_if_folder_exists() {
            let sandbox = create_moon_sandbox("refs");
            sandbox.create_file("types/index.d.ts", "");

            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    toolchain_config: json!({
                        "includeSharedTypes": true,
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
        async fn doesnt_adds_if_disabled() {
            let sandbox = create_moon_sandbox("refs");
            sandbox.create_file("types/index.d.ts", "");

            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    toolchain_config: json!({
                        "includeSharedTypes": false,
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
        async fn doesnt_adds_if_no_folder() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    toolchain_config: json!({
                        "includeSharedTypes": true,
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

    mod include_project_reference_sources {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_includes_from_refs() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "includeProjectReferenceSources": true,
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
        async fn doesnt_add_includes_if_sync_disabled() {
            let sandbox = create_moon_sandbox("refs");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-refs"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "includeProjectReferenceSources": true,
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
    }

    mod route_out_dir_to_cache {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_when_no_options() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-options"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "routeOutDirToCache": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(
                &output,
                "/workspace/no-options/tsconfig.json"
            ));
            assert_snapshot!(
                fs::read_file(sandbox.path().join("no-options/tsconfig.json")).unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_when_has_options() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("with-options"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "routeOutDirToCache": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(
                &output,
                "/workspace/with-options/tsconfig.json"
            ));
            assert_snapshot!(
                fs::read_file(sandbox.path().join("with-options/tsconfig.json")).unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn overrides_out_dir() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("out-dir"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "routeOutDirToCache": true,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(has_changed_file(
                &output,
                "/workspace/out-dir/tsconfig.json"
            ));
            assert_snapshot!(fs::read_file(sandbox.path().join("out-dir/tsconfig.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_when_disabled() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .sync_project(SyncProjectInput {
                    project: create_project("no-options"),
                    project_dependencies: create_project_dependencies(),
                    toolchain_config: json!({
                        "routeOutDirToCache": false,
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!has_changed_file(
                &output,
                "/workspace/no-options/tsconfig.json"
            ));
        }
    }
}
