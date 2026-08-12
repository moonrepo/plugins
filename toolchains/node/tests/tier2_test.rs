use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod node_toolchain_tier2 {
    use super::*;

    mod extend_task_command {
        use super::*;

        // The guest converts virtual paths to real paths by joining the
        // stripped virtual suffix onto the native host prefix with `/`, so on
        // Windows the result is a mixed-separator string like
        // `C:\...\sandbox/project/.moon`. Mirror that exactly, since args are
        // compared as strings, not paths.
        fn expected_prof_dir(root: &std::path::Path) -> String {
            format!("{}/project/.moon", root.display())
        }

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
                    expected_prof_dir(sandbox.path())
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
                    expected_prof_dir(sandbox.path())
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
                    expected_prof_dir(sandbox.path())
                ])
            );
        }
    }
}
