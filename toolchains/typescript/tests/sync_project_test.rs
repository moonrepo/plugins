mod utils;

use moon_pdk::SyncProjectInput;
use moon_pdk_test_utils::create_moon_sandbox;
use serde_json::json;
use starbase_sandbox::assert_snapshot;
use starbase_utils::fs;
use utils::create_project;

mod sync_project {
    use super::*;

    mod create_config {
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

            assert!(
                output
                    .changed_files
                    .iter()
                    .find(
                        |file| file.any_path().as_os_str() == "/workspace/no-config/tsconfig.json")
                    .is_some()
            );
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

            assert!(
                output
                    .changed_files
                    .iter()
                    .find(|file| file.any_path().as_os_str()
                        == "/workspace/no-config/tsconfig.ref.json")
                    .is_some()
            );
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

            assert!(
                output
                    .changed_files
                    .iter()
                    .find(
                        |file| file.any_path().as_os_str() == "/workspace/no-options/tsconfig.json"
                    )
                    .is_none()
            );
            assert!(cfg_path.exists());
        }
    }
}
