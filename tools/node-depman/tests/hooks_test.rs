use proto_pdk_api::RunHook;
use proto_pdk_test_utils::*;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct NodeDepmanPluginConfig {
    pub shared_globals_dir: bool,
}

mod node_depman_tool {
    use super::*;

    mod pre_run {
        use super::*;

        fn create_globals_dir() -> VirtualPath {
            VirtualPath::WithReal {
                path: PathBuf::from("/proto/tools/node/globals/bin"),
                virtual_prefix: PathBuf::from("/proto"),
                real_prefix: PathBuf::from("/.proto"),
            }
        }

        mod npm {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_not_configured() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox.create_plugin("npm-test").await;

                let result = plugin.pre_run(RunHook::default()).await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_disabled() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("npm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: false,
                        });
                    })
                    .await;

                let result = plugin.pre_run(RunHook::default()).await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_enabled_but_no_args() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("npm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_a_prefix_was_provided() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("npm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        passthrough_args: vec![
                            "install".into(),
                            "-g".into(),
                            "typescript".into(),
                            "--prefix".into(),
                            "/some/thing".into(),
                        ],
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn adds_env_var() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("npm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        passthrough_args: vec!["install".into(), "-g".into(), "typescript".into()],
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(result.args, None);
                assert_eq!(
                    result.env,
                    Some(HashMap::from_iter([(
                        "PREFIX".into(),
                        if cfg!(windows) {
                            "/.proto/tools/node/globals/bin".into()
                        } else {
                            "/.proto/tools/node/globals".into()
                        }
                    )]))
                );
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn adds_env_var_with_aliases() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("npm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        passthrough_args: vec![
                            "add".into(),
                            "--global".into(),
                            "typescript".into(),
                        ],
                        ..RunHook::default()
                    })
                    .await;

                assert!(result.env.is_some());
            }
        }

        mod pnpm {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_not_configured() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox.create_plugin("pnpm-test").await;

                let result = plugin.pre_run(RunHook::default()).await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_disabled() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("pnpm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: false,
                        });
                    })
                    .await;

                let result = plugin.pre_run(RunHook::default()).await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_enabled_but_no_args() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("pnpm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_a_dir_was_provided() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("pnpm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        passthrough_args: vec![
                            "add".into(),
                            "-g".into(),
                            "typescript".into(),
                            "--global-dir".into(),
                            "/some/thing".into(),
                        ],
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn adds_args() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("pnpm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        passthrough_args: vec!["add".into(), "-g".into(), "typescript".into()],
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(
                    result.args.as_ref().unwrap().iter().collect::<Vec<_>>(),
                    vec![
                        "--global-dir",
                        "/.proto/tools/node/globals",
                        "--global-bin-dir",
                        "/.proto/tools/node/globals/bin"
                    ]
                );
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn adds_args_with_aliases() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("pnpm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        passthrough_args: vec![
                            "remove".into(),
                            "--global".into(),
                            "typescript".into(),
                        ],
                        ..RunHook::default()
                    })
                    .await;

                assert!(result.args.is_some());
            }
        }

        mod yarn {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_not_configured() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox.create_plugin("yarn-test").await;

                let result = plugin.pre_run(RunHook::default()).await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_disabled() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("yarn-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: false,
                        });
                    })
                    .await;

                let result = plugin.pre_run(RunHook::default()).await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_enabled_but_no_args() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("yarn-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_a_prefix_was_provided() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("yarn-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        passthrough_args: vec![
                            "global".into(),
                            "add".into(),
                            "typescript".into(),
                            "--prefix".into(),
                            "/some/thing".into(),
                        ],
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(result.args, None);
                assert_eq!(result.env, None);
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn adds_env_var() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("yarn-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .pre_run(RunHook {
                        globals_dir: Some(create_globals_dir()),
                        passthrough_args: vec!["global".into(), "add".into(), "typescript".into()],
                        ..RunHook::default()
                    })
                    .await;

                assert_eq!(result.args, None);
                assert_eq!(
                    result.env,
                    Some(HashMap::from_iter([(
                        "PREFIX".into(),
                        "/.proto/tools/node/globals".into()
                    )]))
                );
            }
        }
    }
}
