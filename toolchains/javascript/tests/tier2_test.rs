use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

mod javascript_toolchain_tier2 {
    use super::*;

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
                    starting_dir: VirtualPath::Real(sandbox.path().join("package").into()),
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
}
