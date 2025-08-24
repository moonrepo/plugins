use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod bun_toolchain_tier2 {
    use super::*;

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("bun").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".bun-global"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".bun-global")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prepends_exec_args_when_bun() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("bun").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "bun".into(),
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
        async fn doesnt_prepend_exec_args_when_not_bun() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("bun").await;

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
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("bun").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".bun/global"))),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".bun/global")]);
        }
    }
}
