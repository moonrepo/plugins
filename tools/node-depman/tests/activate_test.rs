use proto_pdk_api::ActivateEnvironmentInput;
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

    mod activate_environment {
        use super::*;

        fn create_globals_dir() -> VirtualPath {
            VirtualPath::Virtual {
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

                let result = plugin
                    .activate_environment(ActivateEnvironmentInput::default())
                    .await;

                assert!(result.env.is_empty());
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

                let result = plugin
                    .activate_environment(ActivateEnvironmentInput::default())
                    .await;

                assert!(result.env.is_empty());
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
                    .activate_environment(ActivateEnvironmentInput {
                        globals_dir: Some(create_globals_dir()),
                        ..ActivateEnvironmentInput::default()
                    })
                    .await;

                assert_eq!(
                    result.env,
                    HashMap::from_iter([(
                        "PREFIX".into(),
                        if cfg!(windows) {
                            "/.proto/tools/node/globals/bin".into()
                        } else {
                            "/.proto/tools/node/globals".into()
                        }
                    )])
                );
            }
        }

        mod pnpm {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_not_configured() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox.create_plugin("pnpm-test").await;

                let result = plugin
                    .activate_environment(ActivateEnvironmentInput::default())
                    .await;

                assert!(result.env.is_empty());
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

                let result = plugin
                    .activate_environment(ActivateEnvironmentInput::default())
                    .await;

                assert!(result.env.is_empty());
            }

            #[tokio::test(flavor = "multi_thread")]

            async fn adds_env_var() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("pnpm-test", |config| {
                        config.tool_config(NodeDepmanPluginConfig {
                            shared_globals_dir: true,
                        });
                    })
                    .await;

                let result = plugin
                    .activate_environment(ActivateEnvironmentInput {
                        globals_dir: Some(create_globals_dir()),
                        ..ActivateEnvironmentInput::default()
                    })
                    .await;

                assert_eq!(
                    result.env,
                    HashMap::from_iter([
                        (
                            "pnpm_config_global_dir".into(),
                            "/.proto/tools/node/globals".into()
                        ),
                        (
                            "pnpm_config_global_bin_dir".into(),
                            "/.proto/tools/node/globals/bin".into()
                        )
                    ])
                );
            }
        }

        mod yarn {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]

            async fn does_nothing_if_not_configured() {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox.create_plugin("yarn-test").await;

                let result = plugin
                    .activate_environment(ActivateEnvironmentInput::default())
                    .await;

                assert!(result.env.is_empty());
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

                let result = plugin
                    .activate_environment(ActivateEnvironmentInput::default())
                    .await;

                assert!(result.env.is_empty());
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
                    .activate_environment(ActivateEnvironmentInput {
                        globals_dir: Some(create_globals_dir()),
                        ..ActivateEnvironmentInput::default()
                    })
                    .await;

                assert_eq!(
                    result.env,
                    HashMap::from_iter([("PREFIX".into(), "/.proto/tools/node/globals".into())])
                );
            }
        }
    }
}
