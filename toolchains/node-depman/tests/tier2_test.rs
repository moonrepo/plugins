use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod node_depman_toolchain_tier2 {
    use super::*;

    mod setup_environment {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_for_npm() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("npm").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_for_pnpm() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("pnpm").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_for_yarn_if_plugins_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("yarn").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "plugins": [],
                        "version": "^2"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "version": "^2"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_for_yarn1_even_with_plugins() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("yarn").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "plugins": ["example"],
                        "version": "^1"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_plugin_commands_for_yarn2() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("yarn").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "plugins": ["foo", "bar"],
                        "version": "^2"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                [
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["plugin", "import", "foo"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    ),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["plugin", "import", "bar"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                ]
            );
        }
    }

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("npm").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".npm-global"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".npm-global")]);
        }
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("yarn").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".yarn/global"))),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".yarn/global")]);
        }
    }
}
