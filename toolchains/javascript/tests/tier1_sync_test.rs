use moon_pdk_api::*;
use moon_pdk_test_utils::create_moon_sandbox;
use serde_json::json;

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
}
