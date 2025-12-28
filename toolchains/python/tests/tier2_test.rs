use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::collections::BTreeMap;

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

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new(
                        "uv",
                        [
                            "sync",
                            "--no-managed-python",
                            "--no-python-downloads",
                            "--no-progress",
                        ]
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("uv", ["sync", "-a", "b", "--c"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv_with_focused_projects() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    packages: vec!["foo".into(), "bar".into()],
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new(
                        "uv",
                        [
                            "sync",
                            "--package",
                            "foo",
                            "--package",
                            "bar",
                            "--no-managed-python",
                            "--no-python-downloads",
                            "--no-progress",
                        ]
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }
    }

    mod parse_lock {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_requirements_txt() {
            let sandbox = create_moon_sandbox("lockfiles");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("requirements.txt")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    ("pytest".into(), vec![LockDependency::default()]),
                    ("pytest-cov".into(), vec![LockDependency::default()]),
                    (
                        "docopt".into(),
                        vec![LockDependency {
                            version: Some(VersionSpec::parse("0.6.1").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "requests".into(),
                        vec![LockDependency {
                            meta: Some("security".into()),
                            ..Default::default()
                        }]
                    ),
                    ("urllib3".into(), vec![LockDependency::default()]),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_pylock_toml() {
            use starbase_sandbox::pretty_assertions::assert_eq;

            let sandbox = create_moon_sandbox("lockfiles");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("pylock.toml")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "attrs".into(),
                        vec![LockDependency {
                            hash: Some(
                                "c75a69e28a550a7e93789579c22aa26b0f5b83b75dc4e08fe092980051e1090a"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("25.1.0").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "cattrs".into(),
                        vec![LockDependency {
                            hash: Some(
                                "67c7495b760168d931a10233f979b28dc04daf853b30752246f4f8471c6d68d0"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("24.1.2").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "numpy".into(),
                        vec![LockDependency {
                            hash: Some(
                                "83807d445817326b4bcdaaaf8e8e9f1753da04341eceec705c001ff342002e5d"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("2.2.3").unwrap()),
                            ..Default::default()
                        }]
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_uv_lock() {
            use starbase_sandbox::pretty_assertions::assert_eq;

            let sandbox = create_moon_sandbox("lockfiles");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("uv.lock")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "anyio".into(),
                        vec![LockDependency {
                            hash: Some(
                                "5aadc6a1bbb7cdb0bede386cac5e2940f5e2ff3aa20277e991cf028e0585ce94"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("4.4.0").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "argcomplete".into(),
                        vec![LockDependency {
                            hash: Some(
                                "4349400469dccfb7950bb60334a680c58d88699bff6159df61251878dc6bf74b"
                                    .into()
                            ),
                            version: Some(VersionSpec::parse("3.5.0").unwrap()),
                            ..Default::default()
                        }]
                    ),
                ])
            );
        }
    }
}
