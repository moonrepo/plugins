use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

mod rust_toolchain_tier2 {
    use super::*;

    mod add_msrv_constraint {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_version() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "addMsrvConstraint": true,
                        "version": null
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("Cargo.toml"))
                    .unwrap()
                    .contains("rust-version")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_disabled() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "addMsrvConstraint": false,
                        "version": "1.69.0"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("Cargo.toml"))
                    .unwrap()
                    .contains("rust-version")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn writes_version() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "addMsrvConstraint": true,
                        "version": "1.69.0"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(
                output
                    .operations
                    .iter()
                    .any(|op| op.id == "add-msrv-constraint")
            );
            assert_eq!(
                output.changed_files,
                [PathBuf::from("/workspace/Cargo.toml")]
            );
            assert!(
                fs::read_to_string(sandbox.path().join("Cargo.toml"))
                    .unwrap()
                    .contains("rust-version = \"1.69.0\"")
            );
        }
    }

    mod sync_toolchain_config {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_version() {
            let sandbox = create_moon_sandbox("tc-cfg");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncToolchainConfig": true,
                        "version": null
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!output.operations.is_empty()); // Always runs
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("rust-toolchain.toml"))
                    .unwrap()
                    .contains("channel")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_disabled() {
            let sandbox = create_moon_sandbox("tc-cfg");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncToolchainConfig": false,
                        "version": "1.69.0"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
            assert!(
                !fs::read_to_string(sandbox.path().join("rust-toolchain.toml"))
                    .unwrap()
                    .contains("channel")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn writes_version() {
            let sandbox = create_moon_sandbox("tc-cfg");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncToolchainConfig": true,
                        "version": "1.69.0"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(
                output
                    .operations
                    .iter()
                    .any(|op| op.id == "sync-toolchain-config")
            );
            assert_eq!(
                output.changed_files,
                [PathBuf::from("/workspace/rust-toolchain.toml")]
            );
            assert!(
                fs::read_to_string(sandbox.path().join("rust-toolchain.toml"))
                    .unwrap()
                    .contains("channel = \"1.69.0\"")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_legacy_config() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("rust-toolchain", "1.0.0");

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncToolchainConfig": true,
                        "version": null
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.changed_files,
                [
                    PathBuf::from("/workspace/rust-toolchain.toml"),
                    PathBuf::from("/workspace/rust-toolchain")
                ]
            );
            assert!(
                fs::read_to_string(sandbox.path().join("rust-toolchain.toml"))
                    .unwrap()
                    .contains("channel = \"1.0.0\"")
            );
            assert!(!sandbox.path().join("rust-toolchain").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_legacy_config_with_channel() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("rust-toolchain", "[toolchain]\nchannel = \"1.0.0\"");

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "syncToolchainConfig": true,
                        "version": null
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.changed_files,
                [
                    PathBuf::from("/workspace/rust-toolchain.toml"),
                    PathBuf::from("/workspace/rust-toolchain")
                ]
            );
            assert!(
                fs::read_to_string(sandbox.path().join("rust-toolchain.toml"))
                    .unwrap()
                    .contains("channel = \"1.0.0\"")
            );
            assert!(!sandbox.path().join("rust-toolchain").exists());
        }
    }

    mod components {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_command_if_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "components": []
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_command_if_not_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "components": ["rustfmt", "clippy"]
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                [ExecCommand::new(
                    ExecCommandInput::new("rustup", ["component", "add", "rustfmt", "clippy"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
                .cache("rustup-component-add")]
            );
        }
    }

    mod targets {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_command_if_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "targets": []
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_command_if_not_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "targets": ["wasm32-wasi", "nightly"]
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                [ExecCommand::new(
                    ExecCommandInput::new("rustup", ["target", "add", "wasm32-wasi", "nightly"],)
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
                .cache("rustup-target-add")]
            );
        }
    }

    mod bins {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_command_if_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
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
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "bins": [
                            "cargo-nextest",
                            {
                                "bin": "just@1"
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
                        ExecCommandInput::new("cargo", ["install", "cargo-binstall", "--force"],)
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("cargo-binstall"),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "cargo",
                            [
                                "binstall",
                                "--no-confirm",
                                "--log-level",
                                "info",
                                "cargo-nextest",
                                "just@1",
                            ],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("cargo-bins")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn separates_forced_and_non_forced_bins() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "bins": [
                            "cargo-nextest",
                            {
                                "bin": "just",
                                "force": true
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
                        ExecCommandInput::new("cargo", ["install", "cargo-binstall", "--force"],)
                            .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("cargo-binstall"),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "cargo",
                            [
                                "binstall",
                                "--no-confirm",
                                "--log-level",
                                "info",
                                "--force",
                                "just",
                            ],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("cargo-bins-forced"),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "cargo",
                            [
                                "binstall",
                                "--no-confirm",
                                "--log-level",
                                "info",
                                "cargo-nextest",
                            ],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("cargo-bins")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_local_bins_when_ci() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox
                .create_toolchain_with_config("rust", |config| {
                    config.host_environment(HostEnvironment {
                        ci: true,
                        ..Default::default()
                    });
                })
                .await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "bins": [
                            {
                                "bin": "just",
                                "local": true
                            }
                        ]
                    }),
                    ..Default::default()
                })
                .await;

            // binstall command
            assert_eq!(output.commands.len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_set_binstall_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "binstallVersion": "1.2.3",
                        "bins": ["cargo-nextest"]
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                [
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "cargo",
                            ["install", "cargo-binstall@1.2.3", "--force"],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("cargo-binstall"),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "cargo",
                            [
                                "binstall",
                                "--no-confirm",
                                "--log-level",
                                "info",
                                "cargo-nextest",
                            ],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("cargo-bins")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_install_binstall_if_a_cargo_bin_exists() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".cargo-bins/cargo-binstall", "");
            sandbox.create_file(".cargo-bins/cargo-binstall.exe", "");

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".cargo-bins"))),
                    toolchain_config: json!({
                        "bins": ["cargo-nextest"]
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                [ExecCommand::new(
                    ExecCommandInput::new(
                        "cargo",
                        [
                            "binstall",
                            "--no-confirm",
                            "--log-level",
                            "info",
                            "cargo-nextest",
                        ],
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
                .cache("cargo-bins")]
            );
        }
    }
}
