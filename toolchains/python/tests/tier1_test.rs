use moon_config::DockerPruneConfig;
use moon_pdk_api::*;
use moon_pdk_test_utils::{MoonWasmSandbox, create_empty_moon_sandbox};
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

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_more_commands_and_version_files() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            for name in ["pip3", "uvx"] {
                assert!(plugin.metadata.exe_names.contains(&name.to_string()));
            }

            assert!(
                plugin
                    .metadata
                    .config_file_globs
                    .contains(&".python-versions".to_string())
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
        async fn copies_setuptools_configs() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.scaffold_globs.contains(&"setup.py".to_string()));
            assert!(output.scaffold_globs.contains(&"setup.cfg".to_string()));
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

    mod scaffold_docker {
        use super::*;

        async fn scaffold(
            sandbox: &MoonWasmSandbox,
            phase: ScaffoldDockerPhase,
            project: Option<ProjectFragment>,
            toolchain_config: serde_json::Value,
        ) -> Vec<VirtualPath> {
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::new(sandbox.path().join("in")),
                    output_dir: VirtualPath::new(sandbox.path().join("out")),
                    phase,
                    project,
                    toolchain_config,
                    ..Default::default()
                })
                .await;

            for file in &output.copied_files {
                let path = sandbox
                    .path()
                    .join(file.strip_prefix("/workspace").unwrap());

                assert!(path.exists());
                assert_eq!(std::fs::read_to_string(path).unwrap(), "");
            }

            output.copied_files
        }

        async fn scaffold_configs(sandbox: &MoonWasmSandbox) -> Vec<VirtualPath> {
            scaffold(
                sandbox,
                ScaffoldDockerPhase::Configs,
                Some(ProjectFragment::default()),
                json!({}),
            )
            .await
        }

        fn stubs(paths: &[&str]) -> Vec<VirtualPath> {
            paths
                .iter()
                .map(|path| VirtualPath::new(format!("/workspace/out/{path}")))
                .collect()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn stubs_a_uv_lib() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "in/pyproject.toml",
                r#"
[project]
name = "b"
version = "0.1.0"
readme = "README.md"

[build-system]
requires = ["uv_build>=0.13.8,<0.14.0"]
build-backend = "uv_build"
"#,
            );
            sandbox.create_file("in/README.md", "# b");
            sandbox.create_file("in/src/b/__init__.py", "def hello(): ...");
            sandbox.create_file("in/src/b/py.typed", "");
            sandbox.create_file("in/src/b/core.py", "");

            assert_eq!(
                scaffold_configs(&sandbox).await,
                stubs(&["README.md", "src/b/__init__.py"])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn stubs_hatchling_readme_and_license_files() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "in/pyproject.toml",
                r#"
[project]
name = "proj"
version = "0.1.0"
readme = { file = "docs/README.rst", content-type = "text/x-rst" }
license = "MIT"
license-files = ["LICEN[CS]E*", "licenses/*.txt"]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
"#,
            );
            sandbox.create_file("in/docs/README.rst", "proj");
            sandbox.create_file("in/LICENSE", "MIT");
            sandbox.create_file("in/licenses/vendored.txt", "BSD");
            sandbox.create_file("in/proj/__init__.py", "");

            assert_eq!(
                scaffold_configs(&sandbox).await,
                stubs(&[
                    "LICENSE",
                    "docs/README.rst",
                    "licenses/vendored.txt",
                    "proj/__init__.py"
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn stubs_a_poetry_src_layout() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "in/pyproject.toml",
                r#"
[project]
name = "proj"
version = "0.1.0"
readme = "README.md"
license = { file = "LICENSE" }

[build-system]
requires = ["poetry-core>=2.0.0,<3.0.0"]
build-backend = "poetry.core.masonry.api"
"#,
            );
            sandbox.create_file("in/README.md", "");
            sandbox.create_file("in/LICENSE", "MIT");
            sandbox.create_file("in/src/proj/__init__.py", "");
            sandbox.create_file("in/src/proj/sub/__init__.py", "");
            sandbox.create_file("in/tests/__init__.py", "");

            assert_eq!(
                scaffold_configs(&sandbox).await,
                stubs(&[
                    "LICENSE",
                    "README.md",
                    "src/proj/__init__.py",
                    "src/proj/sub/__init__.py",
                    "tests/__init__.py"
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_venvs_and_missing_files() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "in/pyproject.toml",
                "[project]\nname = \"a\"\nreadme = \"README.md\"\nlicense-files = [\"LICENSE\"]\n",
            );
            sandbox.create_file("in/a/__init__.py", "");
            sandbox.create_file("in/.venv/lib/python3.14/site-packages/six/__init__.py", "");
            sandbox.create_file("in/sub/.venv/lib/x/__init__.py", "");

            assert_eq!(scaffold_configs(&sandbox).await, stubs(&["a/__init__.py"]));

            std::fs::remove_dir_all(sandbox.path().join("out")).unwrap();
            sandbox.create_file("in/env/lib/x/__init__.py", "");

            assert_eq!(
                scaffold(
                    &sandbox,
                    ScaffoldDockerPhase::Configs,
                    Some(ProjectFragment::default()),
                    json!({ "venvName": "env" }),
                )
                .await,
                stubs(&[
                    ".venv/lib/python3.14/site-packages/six/__init__.py",
                    "a/__init__.py",
                    "sub/.venv/lib/x/__init__.py"
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_copied_files() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("in/a/__init__.py", "x = 1");
            sandbox.create_file("out/a/__init__.py", "x = 1");

            assert!(scaffold_configs(&sandbox).await.is_empty());
            assert_eq!(
                std::fs::read_to_string(sandbox.path().join("out/a/__init__.py")).unwrap(),
                "x = 1"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_in_the_sources_phase_or_workspace() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("in/pyproject.toml", "[project]\nname = \"a\"\n");
            sandbox.create_file("in/a/__init__.py", "");

            assert!(
                scaffold(
                    &sandbox,
                    ScaffoldDockerPhase::Sources,
                    Some(ProjectFragment::default()),
                    json!({}),
                )
                .await
                .is_empty()
            );
            assert!(
                scaffold(&sandbox, ScaffoldDockerPhase::Configs, None, json!({}))
                    .await
                    .is_empty()
            );
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
