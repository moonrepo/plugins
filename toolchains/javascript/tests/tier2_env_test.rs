use moon_pdk_api::*;
use moon_pdk_test_utils::create_moon_sandbox;
use serde_json::json;
use std::fs;

mod javascript_toolchain_tier2 {
    use super::*;

    mod sync_package_manager_field {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_not_enabled() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                assert_eq!(id, "npm");
                json!({ "version": "1.2.3" })
            });

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
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
                .mock_load_toolchain_config(|_, _| json!({ "version": "1.2.3" }));

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
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

            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                assert_eq!(id, "npm");
                json!({})
            });

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
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

            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                assert_eq!(id, "npm");
                json!({ "version": "1" })
            });

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
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

            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                assert_eq!(id, "bun");
                json!({ "version": "1.2.3" })
            });

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
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
        async fn does_nothing_if_deno() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                assert_eq!(id, "deno");
                json!({ "version": "1.2.3" })
            });

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": "deno"
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
        async fn sets_field_for_nub() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                assert_eq!(id, "nub");
                json!({ "version": "1.2.3" })
            });

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": "nub"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/package.json")]
            );
            assert!(
                fs::read_to_string(sandbox.path().join("package.json"))
                    .unwrap()
                    .contains(r#""packageManager": "nub@1.2.3""#)
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_field() {
            let mut sandbox = create_moon_sandbox("files");

            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                assert_eq!(id, "npm");
                json!({ "version": "1.2.3" })
            });

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": "npm"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/package.json")]
            );
            assert!(
                fs::read_to_string(sandbox.path().join("package.json"))
                    .unwrap()
                    .contains(r#""packageManager": "npm@1.2.3""#)
            );
        }

        async fn sync_with_dev_engines(
            package_manager: &str,
            package: serde_json::Value,
        ) -> String {
            let mut sandbox = create_moon_sandbox("files");
            sandbox.create_file("package.json", package.to_string());

            let id = package_manager.to_owned();

            sandbox
                .host_funcs
                .mock_load_toolchain_config(move |toolchain, _| {
                    assert_eq!(toolchain, id);
                    json!({ "version": "10.33.4" })
                });

            let plugin = sandbox.create_toolchain("javascript").await;

            plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "syncPackageManagerField": true,
                        "packageManager": package_manager
                    }),
                    ..Default::default()
                })
                .await;

            fs::read_to_string(sandbox.path().join("package.json")).unwrap()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_dev_engines_instead_of_adding_field() {
            let content = sync_with_dev_engines(
                "pnpm",
                json!({
                    "devEngines": {
                        "packageManager": { "name": "pnpm", "version": "^10.0.0", "onFail": "error" }
                    }
                }),
            )
            .await;
            let package: serde_json::Value = serde_json::from_str(&content).unwrap();

            assert_eq!(
                package["devEngines"]["packageManager"],
                json!({ "name": "pnpm", "version": "10.33.4", "onFail": "error" })
            );
            assert!(package.get("packageManager").is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_dev_engines_and_an_existing_field() {
            let content = sync_with_dev_engines(
                "pnpm",
                json!({
                    "packageManager": "pnpm@10.0.0",
                    "devEngines": {
                        "packageManager": [
                            { "name": "npm", "version": "^11.0.0" },
                            { "name": "pnpm", "version": "^10.0.0" }
                        ]
                    }
                }),
            )
            .await;
            let package: serde_json::Value = serde_json::from_str(&content).unwrap();

            assert_eq!(
                package["devEngines"]["packageManager"][1]["version"],
                "10.33.4"
            );
            assert_eq!(package["packageManager"], "pnpm@10.33.4");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_field_when_dev_engines_names_another() {
            let content = sync_with_dev_engines(
                "pnpm",
                json!({
                    "devEngines": { "packageManager": { "name": "npm", "version": "^11.0.0" } }
                }),
            )
            .await;

            assert!(content.contains(r#""packageManager": "pnpm@10.33.4""#));
            assert!(content.contains(r#""version": "^11.0.0""#));
        }

        // Yarn Switch only reads `packageManager`
        #[tokio::test(flavor = "multi_thread")]
        async fn sets_field_for_yarn() {
            let content = sync_with_dev_engines(
                "yarn",
                json!({
                    "devEngines": { "packageManager": { "name": "yarn", "version": "^4.0.0" } }
                }),
            )
            .await;

            assert!(content.contains(r#""packageManager": "yarn@10.33.4""#));
            assert!(content.contains(r#""version": "^4.0.0""#));
        }
    }
}
