use moon_config::DockerPruneConfig;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod python_toolchain_tier1 {
    use super::*;

    mod register_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn puts_the_source_of_truth_lockfile_first() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            // moon only hashes the first lockfile that exists
            assert_eq!(
                plugin.metadata.lock_file_names,
                [
                    "uv.lock",
                    "poetry.lock",
                    "pylock.toml",
                    "requirements.txt",
                    "Pipfile.lock",
                    "constraints.txt"
                ]
            );
        }
    }

    mod initialize_toolchain {
        use super::*;

        async fn detect(files: &[(&str, &str)]) -> InitializeToolchainOutput {
            let sandbox = create_empty_moon_sandbox();

            for (file, content) in files {
                sandbox.create_file(file, content);
            }

            let plugin = sandbox.create_toolchain("python").await;

            plugin
                .initialize_toolchain(InitializeToolchainInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                })
                .await
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_from_pyproject_tool_tables() {
            for (content, expected) in [
                ("[tool.uv]\ndev-dependencies = []\n", "uv"),
                ("[tool.poetry]\nname = \"a\"\n", "poetry"),
                // uv first, like its files
                ("[tool.poetry]\nname = \"a\"\n\n[tool.uv]\n", "uv"),
            ] {
                let output = detect(&[("pyproject.toml", content), ("requirements.txt", "")]).await;

                assert_eq!(
                    output.default_settings.get("packageManager"),
                    Some(&json!(expected))
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_pip_from_requirements() {
            let output = detect(&[("requirements.txt", "")]).await;

            assert_eq!(
                output.default_settings.get("packageManager"),
                Some(&json!("pip"))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prompts_for_pipfiles() {
            let output = detect(&[("Pipfile", ""), ("Pipfile.lock", "")]).await;

            assert!(!output.default_settings.contains_key("packageManager"));
            assert_eq!(output.prompts.len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn points_to_the_config_docs() {
            let output = detect(&[]).await;

            assert_eq!(
                output.config_url.as_deref(),
                Some("https://moonrepo.dev/docs/config/toolchain#python")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_poetry_as_prompt_option() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                })
                .await;

            assert_eq!(
                output.prompts,
                vec![SettingPrompt::new(
                    "packageManager",
                    "Package manager to install dependencies with?",
                    PromptType::Select {
                        default_index: 0,
                        options: vec![json!("pip"), json!("poetry"), json!("uv"), json!("uv-pip")],
                    },
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_poetry() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("poetry.lock", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                })
                .await;

            assert_eq!(
                output.default_settings.get("packageManager"),
                Some(&json!("poetry"))
            );
        }
    }

    mod define_docker_metadata {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_python_tags() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            for (version, expected) in [
                ("3.12.1", "python:3.12.1"),
                ("3.12.1+20240107", "python:3.12.1"),
                ("3.15.0-rc.2", "python:3.15.0rc2"),
                ("3.15.0-alpha.5", "python:3.15.0a5"),
                ("stable", "python:latest"),
                ("^3.12", "python:3.12"),
            ] {
                let output = plugin
                    .define_docker_metadata(DefineDockerMetadataInput {
                        toolchain_config: json!({ "version": version }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(output.default_image.unwrap(), expected);
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn excludes_venvs_from_scaffolding() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.scaffold_globs.contains(&"!.venv/**/*".to_string()));
            assert!(
                output
                    .scaffold_globs
                    .contains(&"!**/.venv/**/*".to_string())
            );

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({ "venvName": "venv" }),
                    ..Default::default()
                })
                .await;

            assert!(output.scaffold_globs.contains(&"!venv/**/*".to_string()));
            assert!(output.scaffold_globs.contains(&"!**/venv/**/*".to_string()));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_image_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "python:latest");

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({
                        "version": "3.10"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "python:3.10");
        }
    }

    mod prune_docker {
        use super::*;

        fn create_project_fragment(id: &str) -> ProjectFragment {
            ProjectFragment {
                id: Id::raw(id),
                source: id.into(),
                ..Default::default()
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_disabled() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("a/.venv/pyvenv.cfg", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: false,
                        ..Default::default()
                    },
                    project_dependencies: vec![create_project_fragment("a")],
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(sandbox.path().join("a/.venv").exists());

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_venv_dirs() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    project_dependencies: vec![create_project_fragment("a")],
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_venv_dirs_from_dependency_projects() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("a/.venv/pyvenv.cfg", "");
            sandbox.create_file("b/.venv/pyvenv.cfg", "");
            sandbox.create_file("c/.venv/pyvenv.cfg", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    project_dependencies: vec![
                        create_project_fragment("a"),
                        create_project_fragment("b"),
                    ],
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(!sandbox.path().join("a/.venv").exists());
            assert!(!sandbox.path().join("b/.venv").exists());

            // Not a dependency, so remains
            assert!(sandbox.path().join("c/.venv").exists());

            assert_eq!(
                output.changed_files,
                [
                    VirtualPath::new("/workspace/a/.venv"),
                    VirtualPath::new("/workspace/b/.venv")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_custom_named_venv_dirs() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("a/venv/pyvenv.cfg", "");
            sandbox.create_file("a/.venv/pyvenv.cfg", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    project_dependencies: vec![create_project_fragment("a")],
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "venvName": "venv"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!sandbox.path().join("a/venv").exists());

            // Not the configured name, so remains
            assert!(sandbox.path().join("a/.venv").exists());

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/a/venv")]
            );
        }
    }
}
