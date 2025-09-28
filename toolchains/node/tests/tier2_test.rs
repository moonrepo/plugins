use moon_common::path::standardize_separators;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

mod node_toolchain_tier2 {
    use super::*;

    mod setup_environment {
        use super::*;

        mod sync_toolchain_config {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn does_nothing_if_no_version() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("node").await;

                let output = plugin
                    .setup_environment(SetupEnvironmentInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "syncVersionManagerConfig": "nvm",
                            "version": null
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.operations.is_empty());
                assert!(output.changed_files.is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn does_nothing_if_disabled() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("node").await;

                let output = plugin
                    .setup_environment(SetupEnvironmentInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "syncVersionManagerConfig": null,
                            "version": "20.1"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.operations.is_empty());
                assert!(output.changed_files.is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn writes_nvm_version() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("node").await;

                let output = plugin
                    .setup_environment(SetupEnvironmentInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "syncVersionManagerConfig": "nvm",
                            "version": "20.1"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(
                    output
                        .operations
                        .iter()
                        .any(|op| op.id == "sync-version-manager")
                );
                assert_eq!(output.changed_files, [PathBuf::from("/workspace/.nvmrc")]);
                assert_eq!(
                    fs::read_to_string(sandbox.path().join(".nvmrc")).unwrap(),
                    "20.1"
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn writes_nodenv_version() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("node").await;

                let output = plugin
                    .setup_environment(SetupEnvironmentInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "syncVersionManagerConfig": "nodenv",
                            "version": "20.1"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(
                    output
                        .operations
                        .iter()
                        .any(|op| op.id == "sync-version-manager")
                );
                assert_eq!(
                    output.changed_files,
                    [PathBuf::from("/workspace/.node-version")]
                );
                assert_eq!(
                    fs::read_to_string(sandbox.path().join(".node-version")).unwrap(),
                    "20.1"
                );
            }
        }
    }

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn prepends_exec_args_when_node() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("node").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "node".into(),
                    toolchain_config: json!({
                        "executeArgs": ["--test", "-abc"]
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.args.unwrap(),
                Extend::Prepend(vec!["--test".into(), "-abc".into()])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_prepend_exec_args_when_not_node() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("node").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "npm".into(),
                    toolchain_config: json!({
                        "executeArgs": ["--test", "-abc"]
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.args.is_none(),);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prepends_profile_cpu_args_when_node() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("node").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "node".into(),
                    toolchain_config: json!({
                        "profileExecution": "cpu"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.args.unwrap(),
                Extend::Prepend(vec![
                    "--cpu-prof".into(),
                    "--cpu-prof-name".into(),
                    "snapshot.cpuprofile".into(),
                    "--cpu-prof-dir".into(),
                    standardize_separators(sandbox.path().join("project/.moon").to_string_lossy())
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prepends_profile_heap_args_when_node() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("node").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "node".into(),
                    toolchain_config: json!({
                        "profileExecution": "heap"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.args.unwrap(),
                Extend::Prepend(vec![
                    "--heap-prof".into(),
                    "--heap-prof-name".into(),
                    "snapshot.heapprofile".into(),
                    "--heap-prof-dir".into(),
                    standardize_separators(sandbox.path().join("project/.moon").to_string_lossy())
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_prepend_profile_args_when_not_node() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("node").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "npm".into(),
                    toolchain_config: json!({
                        "profileExecution": "cpu"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.args.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prepends_exec_and_profile_args_when_node() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("node").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "node".into(),
                    toolchain_config: json!({
                        "executeArgs": ["--test", "-abc"],
                        "profileExecution": "heap"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.args.unwrap(),
                Extend::Prepend(vec![
                    "--test".into(),
                    "-abc".into(),
                    "--heap-prof".into(),
                    "--heap-prof-name".into(),
                    "snapshot.heapprofile".into(),
                    "--heap-prof-dir".into(),
                    standardize_separators(sandbox.path().join("project/.moon").to_string_lossy())
                ])
            );
        }
    }
}
