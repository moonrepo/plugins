use moon_pdk_api::*;
use moon_pdk_test_utils::create_moon_sandbox;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

mod javascript_toolchain_tier2 {
    use super::*;

    mod sync_package_manager_field {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_not_enabled() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_| json!({ "version": "1.2.3" }));

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncPackageManagerField": false,
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("package.json"))
                    .unwrap()
                    .contains("packageManager")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_pm() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_| json!({ "version": "1.2.3" }));

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": null
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("package.json"))
                    .unwrap()
                    .contains("packageManager")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_pm_version() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox.host_funcs.mock_load_toolchain_config(|_| json!({}));

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("package.json"))
                    .unwrap()
                    .contains("packageManager")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_pm_version_not_semantic() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_| json!({ "version": "1" }));

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("package.json"))
                    .unwrap()
                    .contains("packageManager")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_bun() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_| json!({ "version": "1.2.3" }));

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": "bun"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("package.json"))
                    .unwrap()
                    .contains("packageManager")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_field() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_| json!({ "version": "1.2.3" }));

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.changed_files,
                [PathBuf::from("/workspace/package.json")]
            );
            assert!(
                fs::read_to_string(sandbox.path().join("package.json"))
                    .unwrap()
                    .contains(r#""packageManager": "npm@1.2.3""#)
            );
        }
    }
}
