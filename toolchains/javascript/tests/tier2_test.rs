use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
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

    mod install_dependencies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_no_pm() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "packageManager": null
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.install_command.is_none());
            assert!(output.dedupe_command.is_none());
        }

        mod bun {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "bun",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("bun", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "bun",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "bun",
                            [
                                "install",
                                "--production",
                                "--filter",
                                "foo",
                                "--filter",
                                "@scope/bar"
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_bun_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "bun",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("bun", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert!(output.dedupe_command.is_none());
            }
        }

        mod npm {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "npm",
                            [
                                "install",
                                "--production",
                                "--workspace",
                                "foo",
                                "--workspace",
                                "@scope/bar"
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_npm_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn switches_to_ci_in_ci() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox
                    .create_toolchain_with_config("javascript", |cfg| {
                        cfg.host_environment(HostEnvironment {
                            ci: true,
                            ..Default::default()
                        });
                    })
                    .await;

                // Doesn't work without the lockfile
                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );

                // Now works!
                sandbox.create_file("package-lock.json", "{}");

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "npm",
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("npm", ["ci"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }
        }

        mod pnpm {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("pnpm", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "pnpm",
                            [
                                "install",
                                "--prod",
                                "--filter-prod",
                                "foo...",
                                "--filter-prod",
                                "@scope/bar..."
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_pnpm_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("pnpm", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn legacy_dedupe_for_older_versions() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "npx",
                            [
                                "--quiet",
                                "--package",
                                "pnpm-deduplicate",
                                "--",
                                "pnpm-deduplicate"
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn mordern_dedupe_for_newer_versions() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_| json!({ "version": "8" }));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true,
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("pnpm", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }
        }

        mod yarn {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn default_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["install"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["install", "--production"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn focused_commands_for_berry() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_| json!({ "version": "2" }));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true
                        }),
                        packages: vec!["foo".into(), "@scope/bar".into()],
                        production: true,
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "yarn",
                            ["workspaces", "focus", "foo", "@scope/bar", "--production"]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn inherits_args_from_bun_toolchain() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_| json!({ "installArgs": ["-a", "b", "--c"]}));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.install_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["install", "-a", "b", "--c"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn legacy_dedupe_for_older_versions() {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "pnpm",
                            "dedupeOnLockfileChange": true
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "npx",
                            [
                                "--quiet",
                                "--package",
                                "pnpm-deduplicate",
                                "--",
                                "pnpm-deduplicate"
                            ]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn mordern_dedupe_for_newer_versions() {
                let mut sandbox = create_empty_moon_sandbox();

                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(|_| json!({ "version": "2" }));

                let plugin = sandbox.create_toolchain("javascript").await;

                let output = plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::Real(sandbox.path().into()),
                        toolchain_config: json!({
                            "packageManager": "yarn",
                            "dedupeOnLockfileChange": true,
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.dedupe_command.unwrap(),
                    ExecCommand::new(
                        ExecCommandInput::new("yarn", ["dedupe"])
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                );
            }
        }
    }
}
