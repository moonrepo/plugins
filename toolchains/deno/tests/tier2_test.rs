use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::collections::BTreeMap;
use std::env;

mod deno_toolchain_tier2 {
    use super::*;

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn prepends_exec_args_when_deno() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "deno".into(),
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
        async fn doesnt_prepend_exec_args_when_not_deno() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

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
        async fn inserts_exec_args_after_runtime_subcommands() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

            let extend = |args: &[&str]| {
                plugin.extend_task_command(ExtendTaskCommandInput {
                    command: "deno".into(),
                    args: args.iter().map(|arg| arg.to_string()).collect(),
                    toolchain_config: json!({
                        "executeArgs": ["--allow-env"]
                    }),
                    ..Default::default()
                })
            };
            let args = |args: &[&str]| args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();

            assert_eq!(
                extend(&["run", "-A", "main.ts"]).await.args.unwrap(),
                Extend::Replace(args(&["run", "--allow-env", "-A", "main.ts"]))
            );
            assert_eq!(
                extend(&["test"]).await.args.unwrap(),
                Extend::Replace(args(&["test", "--allow-env"]))
            );
            assert_eq!(
                extend(&["-q", "run", "main.ts"]).await.args.unwrap(),
                Extend::Replace(args(&["-q", "run", "--allow-env", "main.ts"]))
            );
            assert_eq!(
                extend(&["main.ts"]).await.args.unwrap(),
                Extend::Prepend(args(&["--allow-env"]))
            );

            // Other subcommands reject runtime flags
            assert!(extend(&["task", "build"]).await.args.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_the_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;
            let globals_dir = sandbox.path().join("globals");

            unsafe { env::remove_var("DENO_INSTALL_ROOT") };

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    globals_dir: Some(VirtualPath::new(&globals_dir)),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.paths, [globals_dir]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prefers_the_install_root_over_the_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;
            let root = sandbox.path().join("deno");

            unsafe { env::set_var("DENO_INSTALL_ROOT", &root) };

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    globals_dir: Some(VirtualPath::new(sandbox.path().join("globals"))),
                    ..Default::default()
                })
                .await;

            unsafe { env::remove_var("DENO_INSTALL_ROOT") };

            assert_eq!(output.paths, [root.join("bin")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fallsback_to_deno_dir_when_no_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

            unsafe { env::remove_var("DENO_INSTALL_ROOT") };

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                [sandbox.path().join(".home").join(".deno").join("bin")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_install_root_as_is_when_a_bin_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;
            let root = sandbox.path().join("deno").join("bin");

            unsafe { env::set_var("DENO_INSTALL_ROOT", &root) };

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    ..Default::default()
                })
                .await;

            unsafe { env::remove_var("DENO_INSTALL_ROOT") };

            assert_eq!(output.paths, [root]);
        }
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn fallsback_to_deno_dir_when_no_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

            unsafe { env::remove_var("DENO_INSTALL_ROOT") };

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                [sandbox.path().join(".home").join(".deno").join("bin")]
            );
        }
    }

    mod parse_manifest {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("deno").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::new(sandbox.path().join("deno.json")),
                    ..Default::default()
                })
                .await;

            assert!(output.version.is_none());
            assert!(output.dev_dependencies.is_empty());
            assert!(output.build_dependencies.is_empty());
            assert!(output.peer_dependencies.is_empty());
            assert!(!output.publishable);

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("^1.2.3").unwrap()),
                            reference: Some("jsr:@scope/package".into()),
                            ..Default::default()
                        })
                    ),
                    (
                        "b".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("~4.5.6").unwrap()),
                            reference: Some("npm:package".into()),
                            ..Default::default()
                        })
                    ),
                    (
                        "c".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            url: Some("https://deno.land/x/c/mod.ts".into()),
                            ..Default::default()
                        })
                    ),
                    (
                        "d".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            path: Some("./some/long/path/d.ts".into()),
                            ..Default::default()
                        })
                    )
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_mark_as_publishable() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("deno").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::new(sandbox.path().join("deno-publish.json")),
                    ..Default::default()
                })
                .await;

            assert!(output.publishable);
        }
    }

    mod setup_env_bins {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_command_if_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "bins": []
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_commands_if_not_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "bins": [
                            "jsr:@std/http/file-server",
                            {
                                "bin": "https://examples.deno.land/color-logging.ts"
                            }
                        ]
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                [
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "deno",
                            [
                                "install",
                                "--global",
                                "--force",
                                "--allow-net",
                                "--allow-read",
                                "jsr:@std/http/file-server"
                            ],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache(CacheStrategy::Memory)
                    .label("deno-bin-jsr:@std/http/file-server"),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "deno",
                            [
                                "install",
                                "--global",
                                "--force",
                                "--allow-net",
                                "--allow-read",
                                "https://examples.deno.land/color-logging.ts"
                            ],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache(CacheStrategy::Memory)
                    .label("deno-bin-https://examples.deno.land/color-logging.ts"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_customize_name() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "bins": [
                            {
                                "bin": "jsr:@std/http/file-server",
                                "name": "fs"
                            }
                        ]
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                [ExecCommand::new(
                    ExecCommandInput::new(
                        "deno",
                        [
                            "install",
                            "--global",
                            "--force",
                            "--allow-net",
                            "--allow-read",
                            "--name",
                            "fs",
                            "jsr:@std/http/file-server"
                        ],
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
                .cache(CacheStrategy::Memory)
                .label("deno-bin-jsr:@std/http/file-server")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_pass_args() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("deno").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "bins": [
                            {
                                "bin": "jsr:@std/http/file-server",
                                "args": ["--allow-env", "--allow-sys"]
                            }
                        ]
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                [ExecCommand::new(
                    ExecCommandInput::new(
                        "deno",
                        [
                            "install",
                            "--global",
                            "--force",
                            "--allow-net",
                            "--allow-read",
                            "--allow-env",
                            "--allow-sys",
                            "jsr:@std/http/file-server"
                        ],
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
                .cache(CacheStrategy::Memory)
                .label("deno-bin-jsr:@std/http/file-server")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_local_bins_when_ci() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox
                .create_toolchain_with_config("deno", |config| {
                    config.host_environment(HostEnvironment {
                        ci: true,
                        ..Default::default()
                    });
                })
                .await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "bins": [
                            {
                                "bin": "jsr:@std/http/file-server",
                                "local": true
                            }
                        ]
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }
    }
}
