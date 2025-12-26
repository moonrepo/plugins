use moon_common::path::standardize_separators;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

mod python_toolchain_tier2 {
    use super::*;

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_venv_paths_if_no_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "python".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.paths.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_venv_paths_if_dir_exists() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/file", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "python".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                vec![
                    sandbox.path().join(".venv/Scripts"),
                    sandbox.path().join(".venv/bin")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_venv_paths_traversing_upwards() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/file", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    context: MoonContext {
                        working_dir: plugin
                            .plugin
                            .to_virtual_path(sandbox.path().join("sub/dir")),
                        ..plugin.create_context()
                    },
                    command: "python".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                vec![
                    sandbox.path().join(".venv/Scripts"),
                    sandbox.path().join(".venv/bin")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_venv_paths_with_custom_name() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".virtual-env/file", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "python".into(),
                    toolchain_config: json!({
                        "venvName": ".virtual-env"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                vec![
                    sandbox.path().join(".virtual-env/Scripts"),
                    sandbox.path().join(".virtual-env/bin")
                ]
            );
        }
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_venv_paths_if_no_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "python".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.paths.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_venv_paths_if_dir_exists() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/file", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "python".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                vec![
                    sandbox.path().join(".venv/Scripts"),
                    sandbox.path().join(".venv/bin")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_venv_paths_traversing_upwards() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/file", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    context: MoonContext {
                        working_dir: plugin
                            .plugin
                            .to_virtual_path(sandbox.path().join("sub/dir")),
                        ..plugin.create_context()
                    },
                    script: "python".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                vec![
                    sandbox.path().join(".venv/Scripts"),
                    sandbox.path().join(".venv/bin")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_venv_paths_with_custom_name() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".virtual-env/file", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "python".into(),
                    toolchain_config: json!({
                        "venvName": ".virtual-env"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                vec![
                    sandbox.path().join(".virtual-env/Scripts"),
                    sandbox.path().join(".virtual-env/bin")
                ]
            );
        }
    }

    mod install_dependencies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_pm() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.install_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_pip() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("python", ["-m", "pip", "install"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_pip_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("python", ["-m", "pip", "install", "-a", "b", "--c"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }
    }
}
