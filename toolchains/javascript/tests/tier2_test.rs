use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use starbase_utils::fs;
use std::collections::BTreeMap;
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
                    starting_dir: VirtualPath::Real(sandbox.path().join("package")),
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
            async fn modern_dedupe_for_newer_versions() {
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
            async fn modern_dedupe_for_newer_versions() {
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

    mod parse_manifest {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_workspace() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::Real(sandbox.path().join("package.json")),
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
                        ManifestDependency::Version(UnresolvedVersionSpec::parse("1.2.3").unwrap())
                    ),
                    (
                        "b".into(),
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("^4.5.6").unwrap()
                        ),
                    ),
                    (
                        "c".into(),
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("~7.8.9").unwrap()
                        ),
                    )
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::Real(sandbox.path().join("package/package.json")),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.version.unwrap(), Version::parse("1.0.0").unwrap());
            assert!(output.publishable);

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        ManifestDependency::Version(UnresolvedVersionSpec::parse("1.2.3").unwrap())
                    ),
                    (
                        "e".into(),
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("=7.8.9").unwrap()
                        ),
                    )
                ])
            );

            assert_eq!(
                output.dev_dependencies,
                BTreeMap::from_iter([
                    (
                        "b".into(),
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("^4.5.6").unwrap()
                        ),
                    ),
                    ("f".into(), ManifestDependency::path("../other".into()))
                ])
            );

            assert_eq!(
                output.build_dependencies,
                BTreeMap::from_iter([(
                    "c".into(),
                    ManifestDependency::Version(UnresolvedVersionSpec::parse("*").unwrap())
                )])
            );

            assert_eq!(
                output.peer_dependencies,
                BTreeMap::from_iter([(
                    "d".into(),
                    ManifestDependency::Version(UnresolvedVersionSpec::parse(">=1").unwrap())
                )])
            );
        }
    }

    mod parse_lock {
        use super::*;
        use moon_pdk_test_utils::MoonWasmSandbox;

        fn create_lockfile_sandbox(pm: &str) -> MoonWasmSandbox {
            let sandbox = create_moon_sandbox("deps");
            let lockfiles = create_moon_sandbox("lockfiles");

            fs::copy_dir_all(
                lockfiles.path().join(pm),
                lockfiles.path().join(pm),
                sandbox.path(),
            )
            .unwrap();

            sandbox.debug_files();

            sandbox
        }

        fn expected_packages() -> BTreeMap<String, Option<Version>> {
            BTreeMap::from_iter([
                ("a".into(), Some(Version::new(1, 0, 0))),
                ("b".into(), Some(Version::new(2, 0, 0))),
                ("c".into(), Some(Version::new(3, 0, 0))),
            ])
        }

        fn expected_base_dependencies() -> BTreeMap<String, Vec<LockDependency>> {
            BTreeMap::from_iter([
                (
                    "csstype".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-M1uQkMl8rQK/szD0LNhtqxIPLpimGm8sOBwU7lLnCpSbTyY3yeU1Vc7l4KT5zT4s/yOxHH5O7tIuuLOCnLADRw=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("3.1.3").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "react".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-w8nqGImo45dmMIfljjMwOGtbmC/mk4CMYhWIicdSflH91J9TyCyczcPFXJzrZ/ZXcgGRFeP6BU0BEJTw6tZdfQ=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("19.1.1").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "seroval".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-RbcPH1n5cfwKrru7v7+zrZvjLurgHhGyso3HTyGtRivGWgYjbOmGuivCQaORNELjNONoK35nj28EoWul9sb1zQ=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("1.3.2").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "seroval-plugins".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-0QvCV2lM3aj/U3YozDiVwx9zpH0q8A60CTWIv4Jszj/givcudPb48B+rkU5D51NJ0pTpweGMttHjboPa9/zoIQ=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("1.3.2").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "solid-js".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-A0ZBPJQldAeGCTW0YRYJmt7RCeh5rbFfPZ2aOttgYnctHE7HgKeHCBB/PVc2P7eOfmNXqMFFFoYYdm3S4dcbkA=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("1.9.9").unwrap()),
                        ..Default::default()
                    }]
                ),
                (
                    "typescript".into(),
                    vec![LockDependency {
                        hash: Some(
                            "sha512-CWBzXQrc/qOkhidw1OzBTQuYRbfyxDXJMVJ1XNwUHGROVmuaeiEm3OslpZ1RV96d7SKKjZKrSJu3+t/xlw3R9A=="
                                .into()
                        ),
                        version: Some(VersionSpec::parse("5.9.2").unwrap()),
                        ..Default::default()
                    }]
                ),
            ])
        }

        fn expected_dependencies() -> BTreeMap<String, Vec<LockDependency>> {
            let mut map = BTreeMap::from_iter([
                (
                    "a".into(),
                    vec![LockDependency {
                        version: Some(VersionSpec::parse("1.0.0").unwrap()),
                        ..Default::default()
                    }],
                ),
                (
                    "b".into(),
                    vec![LockDependency {
                        version: Some(VersionSpec::parse("2.0.0").unwrap()),
                        ..Default::default()
                    }],
                ),
                (
                    "c".into(),
                    vec![LockDependency {
                        version: Some(VersionSpec::parse("3.0.0").unwrap()),
                        ..Default::default()
                    }],
                ),
            ]);
            map.extend(expected_base_dependencies());
            map
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_bun() {
            let sandbox = create_lockfile_sandbox("bun");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().to_path_buf()),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.packages, expected_packages());
            assert_eq!(output.dependencies, expected_dependencies());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_npm() {
            let sandbox = create_lockfile_sandbox("npm");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().to_path_buf()),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.packages, expected_packages());
            assert_eq!(output.dependencies, expected_dependencies());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_pnpm() {
            let sandbox = create_lockfile_sandbox("pnpm");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().to_path_buf()),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.packages, expected_packages());
            assert_eq!(output.dependencies, expected_dependencies());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_yarn() {
            let sandbox = create_lockfile_sandbox("yarn");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().to_path_buf()),
                    ..Default::default()
                })
                .await;

            dbg!(&output);

            // Workspace packages in the lockfile have their version
            // set to `0.0.0-use.local` instead of the actual version

            assert_eq!(
                output.packages,
                BTreeMap::from_iter([
                    ("a".into(), Version::parse("0.0.0-use.local").ok()),
                    ("b".into(), Version::parse("0.0.0-use.local").ok()),
                    ("c".into(), Version::parse("0.0.0-use.local").ok()),
                ])
            );

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        vec![LockDependency {
                            version: VersionSpec::parse("0.0.0-use.local").ok(),
                            ..Default::default()
                        }],
                    ),
                    (
                        "b".into(),
                        vec![LockDependency {
                            version: VersionSpec::parse("0.0.0-use.local").ok(),
                            ..Default::default()
                        }],
                    ),
                    (
                        "c".into(),
                        vec![LockDependency {
                            version: VersionSpec::parse("0.0.0-use.local").ok(),
                            ..Default::default()
                        }],
                    ),
                    (
                        "csstype".into(),
                        vec![LockDependency {
                            hash: Some(
                                "sha512-gMCJ1vfgxbK9g88FOatBR0GYV5WE+hDYbQyv4GQiAjQ8vBGeB2oLGuzhkZiUdwgUFdZsn++/PJV/wvxLcAnySA=="
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("3.1.3").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "react".into(),
                        vec![LockDependency {
                            hash: Some(
                                "sha512-jJdpot/QLmA69kRQWDJebIoktHsYXQ5GH2amRUdl3crss/CpAYSDbGi7UJ88OCSDWe28QvDQfCPrUApcMMh7Tg=="
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("19.1.1").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "seroval".into(),
                        vec![LockDependency {
                            hash: Some(
                                "sha512-GedIJWQ3htIuXFgFS9KAZSON4BVlRa+6gvmn0+5w6k8CSbQn8xe8a/mDhJ3ejkGQJkco2QyEYgqhY7+8WXHxvA=="
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("1.3.2").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "seroval-plugins".into(),
                        vec![LockDependency {
                            hash: Some(
                                "sha512-Z7EIs8vBiazKRFtRLr134RtVxqo9FhDDoLSCK2PlxtCkQmrG5QV0dyzHQyV/ChaopNEuXk8ootqOH1g7AKJ7vg=="
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("1.3.2").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "solid-js".into(),
                        vec![LockDependency {
                            hash: Some(
                                "sha512-vT/aIrZsSVW7Tykja2Itz7Ebfzfsf6l082YpK+8QdnUPreLmVtWEzZd/UDVLI8/KQCborlFT6ZHIwRKxWyHJ4w=="
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("1.9.9").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "typescript".into(),
                        vec![LockDependency {
                            hash: Some(
                                "sha512-zWNdUPAtbPmO1C3i92KJcBwexYejYzaSVfAe0VqvIr4IEyJr/zxT6Z2XH5tUDgs8x1g9vgX63tSbGwvtL2OKGA=="
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("5.9.2").unwrap()),
                            ..Default::default()
                        }]
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_yarn_classic() {
            let sandbox = create_lockfile_sandbox("yarn-classic");
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().to_path_buf()),
                    ..Default::default()
                })
                .await;

            dbg!(&output);

            assert_eq!(output.packages, expected_packages());
            assert_eq!(output.dependencies, expected_dependencies());
        }
    }
}
