use moon_config::DependencyScope;
use moon_pdk_api::*;
use moon_pdk_test_utils::{MoonWasmSandbox, create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::collections::BTreeMap;

mod python_toolchain_tier2 {
    use super::*;

    mod define_requirements {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn requires_the_package_manager_toolchains() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            for (package_manager, expected) in [
                (json!("pip"), vec!["unstable_pip"]),
                (json!("poetry"), vec!["unstable_poetry"]),
                (json!("uv"), vec!["unstable_uv"]),
                // Install args from pip, and the venv from uv (D1)
                (json!("uv-pip"), vec!["unstable_uv", "unstable_pip"]),
                (json!(null), vec![]),
            ] {
                let output: DefineRequirementsOutput = plugin
                    .plugin
                    .call_func_with(
                        "define_requirements",
                        DefineRequirementsInput {
                            toolchain_config: json!({ "packageManager": package_manager }),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();

                assert_eq!(output.requires, expected);
            }
        }
    }

    mod extend_project_graph {
        use super::*;

        async fn create_graph(
            sandbox: &MoonWasmSandbox,
            package_manager: &str,
            sources: &[(&str, &str)],
        ) -> ExtendProjectGraphOutput {
            let plugin = sandbox.create_toolchain("python").await;
            let mut input = ExtendProjectGraphInput {
                toolchain_config: json!({ "packageManager": package_manager }),
                ..Default::default()
            };

            for (id, source) in sources {
                input.project_sources.insert(Id::raw(*id), (*source).into());
            }

            plugin.extend_project_graph(input).await
        }

        fn get_deps(output: &ExtendProjectGraphOutput, id: &str) -> Vec<(String, DependencyScope)> {
            output
                .extended_projects
                .get(id)
                .unwrap()
                .dependencies
                .iter()
                .map(|dep| (dep.id.to_string(), dep.scope))
                .collect()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn links_dependency_groups_as_development() {
            let sandbox = create_moon_sandbox("projects");
            sandbox.create_file(
                "c/pyproject.toml",
                r#"[project]
name = "c"
dependencies = ["a"]

[dependency-groups]
dev = ["b", { include-group = "lint" }]
lint = []
"#,
            );

            for package_manager in ["pip", "uv"] {
                let output = create_graph(
                    &sandbox,
                    package_manager,
                    &[("a", "a"), ("b", "b"), ("c", "c")],
                )
                .await;

                assert_eq!(
                    get_deps(&output, "c"),
                    [
                        ("a".to_string(), DependencyScope::Production),
                        ("b".to_string(), DependencyScope::Development)
                    ]
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn links_uv_dev_dependencies_only_for_uv() {
            let sandbox = create_moon_sandbox("projects");
            sandbox.create_file(
                "c/pyproject.toml",
                r#"[project]
name = "c"

[tool.uv]
dev-dependencies = ["b"]
"#,
            );

            let output = create_graph(&sandbox, "uv", &[("b", "b"), ("c", "c")]).await;

            assert_eq!(
                get_deps(&output, "c"),
                [("b".to_string(), DependencyScope::Development)]
            );

            let output = create_graph(&sandbox, "pip", &[("b", "b"), ("c", "c")]).await;

            assert!(get_deps(&output, "c").is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_the_first_scope_for_duplicates() {
            let sandbox = create_moon_sandbox("projects");
            sandbox.create_file(
                "c/pyproject.toml",
                r#"[project]
name = "c"
dependencies = ["a"]

[dependency-groups]
dev = ["a"]
"#,
            );

            let output = create_graph(&sandbox, "uv", &[("a", "a"), ("c", "c")]).await;

            assert_eq!(
                get_deps(&output, "c"),
                [("a".to_string(), DependencyScope::Production)]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_sources_from_the_uv_workspace_root() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "pyproject.toml",
                r#"[tool.uv.workspace]
members = ["packages/*"]

[tool.uv.sources]
a = { workspace = true }
lib = { path = "libs/lib" }
"#,
            );
            sandbox.create_file("packages/a/pyproject.toml", "[project]\nname = \"a\"\n");
            sandbox.create_file(
                "packages/b/pyproject.toml",
                "[project]\nname = \"b\"\ndependencies = [\"a>=0.1\", \"lib>=1\"]\n",
            );
            sandbox.create_file("libs/lib/pyproject.toml", "[project]\nname = \"lib\"\n");

            let mut output = create_graph(
                &sandbox,
                "uv",
                &[
                    ("a", "packages/a"),
                    ("b", "packages/b"),
                    ("lib", "libs/lib"),
                ],
            )
            .await;

            assert_eq!(
                get_deps(&output, "b"),
                [
                    ("a".to_string(), DependencyScope::Production),
                    ("lib".to_string(), DependencyScope::Production)
                ]
            );

            output.input_files.sort_by(|a, d| a.cmp(d));

            assert_eq!(
                output.input_files,
                [
                    VirtualPath::new("/workspace/libs/lib/pyproject.toml"),
                    VirtualPath::new("/workspace/packages/a/pyproject.toml"),
                    VirtualPath::new("/workspace/packages/b/pyproject.toml"),
                    VirtualPath::new("/workspace/pyproject.toml"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn links_path_sources_by_path() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "packages/b/pyproject.toml",
                r#"[project]
name = "b"
dependencies = ["shared>=1", "wheel>=1"]

[tool.uv.sources]
shared = { path = "../../libs/shared" }
wheel = { path = "dist/wheel-1.0-py3-none-any.whl" }
"#,
            );
            // The path decides, not the name
            sandbox.create_file(
                "libs/shared/pyproject.toml",
                "[project]\nname = \"shared-utils\"\n",
            );
            sandbox.create_file("wheel/pyproject.toml", "[project]\nname = \"wheel\"\n");

            let output = create_graph(
                &sandbox,
                "uv",
                &[
                    ("b", "packages/b"),
                    ("shared", "libs/shared"),
                    ("wheel", "wheel"),
                ],
            )
            .await;

            assert_eq!(
                get_deps(&output, "b"),
                [("shared".to_string(), DependencyScope::Production)]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn links_poetry_path_dependencies() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "app/pyproject.toml",
                r#"[tool.poetry]
name = "app"
version = "1.0.0"

[tool.poetry.dependencies]
python = "^3.12"
lib = { path = "../lib", develop = true }
multi = [{ version = "1", python = "<3.9" }, { path = "../multi" }]
requests = "^2.31"
wheel = { path = "../dist/wheel-1.0-py3-none-any.whl" }

[tool.poetry.group.dev.dependencies]
testing = { path = "../testing" }
"#,
            );
            sandbox.create_file(
                "lib/pyproject.toml",
                r#"[project]
name = "lib"

[tool.poetry.dev-dependencies]
testing = { path = "../testing" }
"#,
            );
            sandbox.create_file("multi/pyproject.toml", "[project]\nname = \"multi\"\n");
            sandbox.create_file(
                "requests/pyproject.toml",
                "[project]\nname = \"requests\"\n",
            );
            sandbox.create_file(
                "testing/pyproject.toml",
                "[tool.poetry]\nname = \"testing\"\n",
            );

            let sources = [
                ("app", "app"),
                ("lib", "lib"),
                ("multi", "multi"),
                ("requests", "requests"),
                ("testing", "testing"),
            ];
            let output = create_graph(&sandbox, "poetry", &sources).await;

            assert_eq!(
                output
                    .extended_projects
                    .get("app")
                    .unwrap()
                    .alias
                    .as_deref(),
                Some("app")
            );
            assert_eq!(
                get_deps(&output, "app"),
                [
                    ("lib".to_string(), DependencyScope::Production),
                    ("multi".to_string(), DependencyScope::Production),
                    ("testing".to_string(), DependencyScope::Development)
                ]
            );
            assert_eq!(
                get_deps(&output, "lib"),
                [("testing".to_string(), DependencyScope::Development)]
            );

            // Poetry tables are only read for Poetry
            let output = create_graph(&sandbox, "uv", &sources).await;

            assert!(!output.extended_projects.contains_key("app"));
            assert!(get_deps(&output, "lib").is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_requirements_that_cant_be_parsed() {
            let sandbox = create_moon_sandbox("projects");
            sandbox.create_file(
                "c/pyproject.toml",
                "[project]\nname = \"c\"\ndependencies = [\"a\", \"not valid!!\"]\n",
            );

            let output = create_graph(&sandbox, "uv", &[("a", "a"), ("c", "c")]).await;

            assert_eq!(
                get_deps(&output, "c"),
                [("a".to_string(), DependencyScope::Production)]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn loads_for_all_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("python").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("a"), "a".into());
            input.project_sources.insert(Id::raw("b"), "b".into());
            input.project_sources.insert(Id::raw("c"), "c".into());

            let mut output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([
                    (
                        Id::raw("a"),
                        ExtendProjectOutput {
                            alias: Some("a".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("b"),
                        ExtendProjectOutput {
                            alias: Some("b".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        Id::raw("c"),
                        ExtendProjectOutput {
                            alias: Some("c".into()),
                            dependencies: vec![
                                ProjectDependency {
                                    id: Id::raw("a"),
                                    scope: DependencyScope::Production,
                                    via: Some("requirement a".into()),
                                },
                                ProjectDependency {
                                    id: Id::raw("b"),
                                    scope: DependencyScope::Production,
                                    via: Some("requirement b".into()),
                                }
                            ],
                            ..Default::default()
                        }
                    ),
                ])
            );

            output.input_files.sort_by(|a, d| a.cmp(d));

            assert_eq!(
                output.input_files,
                [
                    VirtualPath::new("/workspace/a/pyproject.toml"),
                    VirtualPath::new("/workspace/b/pyproject.toml"),
                    VirtualPath::new("/workspace/c/pyproject.toml"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_requirements_against_normalized_manifest_name() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "backend_service/pyproject.toml",
                r#"[project]
name = "internal_lib"
version = "1.0.0"
"#,
            );
            sandbox.create_file(
                "consumer/pyproject.toml",
                r#"[project]
name = "consumer"
version = "1.0.0"
dependencies = ["internal-lib"]
"#,
            );

            let plugin = sandbox.create_toolchain("python").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert(Id::raw("backend-service"), "backend_service".into());
            input
                .project_sources
                .insert(Id::raw("consumer"), "consumer".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output
                    .extended_projects
                    .get("consumer")
                    .unwrap()
                    .dependencies,
                vec![ProjectDependency {
                    id: Id::raw("backend-service"),
                    scope: DependencyScope::Production,
                    via: Some("requirement internal-lib".into()),
                }]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_projects_not_in_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("python").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("a"), "a".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([(
                    Id::raw("a"),
                    ExtendProjectOutput {
                        alias: Some("a".into()),
                        ..Default::default()
                    }
                ),])
            );

            assert_eq!(
                output.input_files,
                [VirtualPath::new("/workspace/a/pyproject.toml")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_uv_workspace_requirements() {
            let sandbox = create_moon_sandbox("projects");
            sandbox.create_file(
                "c/pyproject.toml",
                r#"[project]
name = "c"
dependencies = ["a>=1,<2", "b[server]>=1,<2"]

[tool.uv.sources]
a = { workspace = true }
B = [{ workspace = true }]
"#,
            );
            let plugin = sandbox.create_toolchain("python").await;

            for package_manager in ["uv", "uv-pip"] {
                let mut input = ExtendProjectGraphInput {
                    toolchain_config: json!({ "packageManager": package_manager }),
                    ..Default::default()
                };
                input.project_sources.insert(Id::raw("a"), "a".into());
                input.project_sources.insert(Id::raw("b"), "b".into());
                input.project_sources.insert(Id::raw("c"), "c".into());

                let output = plugin.extend_project_graph(input).await;

                assert_eq!(
                    output.extended_projects.get("c").unwrap().dependencies,
                    vec![
                        ProjectDependency {
                            id: Id::raw("a"),
                            scope: DependencyScope::Production,
                            via: Some("requirement a".into()),
                        },
                        ProjectDependency {
                            id: Id::raw("b"),
                            scope: DependencyScope::Production,
                            via: Some("requirement b".into()),
                        }
                    ]
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_registry_and_url_requirements() {
            let sandbox = create_moon_sandbox("projects");
            sandbox.create_file(
                "c/pyproject.toml",
                r#"[project]
name = "c"
dependencies = ["a>=1,<2", "b @ https://example.com/b.whl"]

[tool.uv.sources]
a = { index = "custom" }
b = { workspace = true }
"#,
            );
            let plugin = sandbox.create_toolchain("python").await;

            let mut input = ExtendProjectGraphInput {
                toolchain_config: json!({ "packageManager": "uv" }),
                ..Default::default()
            };
            input.project_sources.insert(Id::raw("a"), "a".into());
            input.project_sources.insert(Id::raw("b"), "b".into());
            input.project_sources.insert(Id::raw("c"), "c".into());

            let output = plugin.extend_project_graph(input).await;

            assert!(
                output
                    .extended_projects
                    .get("c")
                    .unwrap()
                    .dependencies
                    .is_empty()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_self_requirements() {
            let sandbox = create_moon_sandbox("projects");
            sandbox.create_file(
                "c/pyproject.toml",
                r#"[project]
name = "c"
version = "1.0.0"

[project.optional-dependencies]
foo = []
all = ["c[foo]>=1"]

[tool.uv.sources]
c = { workspace = true }
"#,
            );
            let plugin = sandbox.create_toolchain("python").await;

            let mut input = ExtendProjectGraphInput {
                toolchain_config: json!({ "packageManager": "uv" }),
                ..Default::default()
            };
            input.project_sources.insert(Id::raw("c"), "c".into());

            let output = plugin.extend_project_graph(input).await;

            assert!(
                output
                    .extended_projects
                    .get("c")
                    .unwrap()
                    .dependencies
                    .is_empty()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_uv_sources_for_pip() {
            let sandbox = create_moon_sandbox("projects");
            sandbox.create_file(
                "c/pyproject.toml",
                r#"[project]
name = "c"
dependencies = ["a", "b>=1,<2"]

[tool.uv.sources]
a = { index = "custom" }
b = { workspace = true }
"#,
            );
            let plugin = sandbox.create_toolchain("python").await;

            let mut input = ExtendProjectGraphInput {
                toolchain_config: json!({ "packageManager": "pip" }),
                ..Default::default()
            };
            input.project_sources.insert(Id::raw("a"), "a".into());
            input.project_sources.insert(Id::raw("b"), "b".into());
            input.project_sources.insert(Id::raw("c"), "c".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects.get("c").unwrap().dependencies,
                vec![ProjectDependency {
                    id: Id::raw("a"),
                    scope: DependencyScope::Production,
                    via: Some("requirement a".into()),
                }]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_projects_without_a_manifest() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("python").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert(Id::raw("no-manifest"), "no-manifest".into());

            let output = plugin.extend_project_graph(input).await;

            assert!(output.extended_projects.is_empty());
            assert!(output.input_files.is_empty());
        }
    }

    mod extend_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_virtual_env() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/file", "");

            let plugin = sandbox.create_toolchain("python").await;

            for package_manager in ["pip", "poetry", "uv"] {
                let output = plugin
                    .extend_command(ExtendCommandInput {
                        command: "python".into(),
                        toolchain_config: json!({ "packageManager": package_manager }),
                        current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output.env.get("VIRTUAL_ENV"),
                    Some(&sandbox.path().join(".venv").to_string_lossy().to_string())
                );
                // `.venv` is already uv's default
                assert!(!output.env.contains_key("UV_PROJECT_ENVIRONMENT"));
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_uv_project_environment_for_custom_names() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("venv/file", "");

            let plugin = sandbox.create_toolchain("python").await;
            let venv = sandbox.path().join("venv").to_string_lossy().to_string();

            let output = plugin
                .extend_command(ExtendCommandInput {
                    command: "uv".into(),
                    toolchain_config: json!({ "packageManager": "uv", "venvName": "venv" }),
                    current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.env.get("VIRTUAL_ENV"), Some(&venv));
            assert_eq!(output.env.get("UV_PROJECT_ENVIRONMENT"), Some(&venv));

            let output = plugin
                .extend_command(ExtendCommandInput {
                    command: "poetry".into(),
                    toolchain_config: json!({ "packageManager": "poetry", "venvName": "venv" }),
                    current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.env.get("VIRTUAL_ENV"), Some(&venv));
            assert!(!output.env.contains_key("UV_PROJECT_ENVIRONMENT"));
        }

        async fn get_uv_python(
            sandbox: &MoonWasmSandbox,
            toolchain_config: serde_json::Value,
        ) -> Option<String> {
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_command(ExtendCommandInput {
                    command: "uv".into(),
                    toolchain_config,
                    current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                    ..Default::default()
                })
                .await;

            output.env.get("UV_PYTHON").cloned()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_uv_python_for_versions_without_a_range() {
            let sandbox = create_empty_moon_sandbox();

            for package_manager in ["uv", "uv-pip"] {
                for (version, expected) in [
                    ("3.14.6", "3.14.6"),
                    ("3.14.6+20260101", "3.14.6"),
                    ("3.15.0-rc.2", "3.15.0rc2"),
                    ("3.14", "3.14"),
                ] {
                    assert_eq!(
                        get_uv_python(
                            &sandbox,
                            json!({ "packageManager": package_manager, "version": version })
                        )
                        .await
                        .as_deref(),
                        Some(expected)
                    );
                }
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_set_uv_python_for_ranges_aliases_or_other_managers() {
            let sandbox = create_empty_moon_sandbox();

            for toolchain_config in [
                json!({ "packageManager": "uv", "version": "^3.12" }),
                json!({ "packageManager": "uv", "version": "stable" }),
                json!({ "packageManager": "uv" }),
                json!({ "packageManager": "pip", "version": "3.14.6" }),
                json!({ "packageManager": "poetry", "version": "3.14.6" }),
            ] {
                assert_eq!(get_uv_python(&sandbox, toolchain_config).await, None);
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_set_env_without_a_venv() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_command(ExtendCommandInput {
                    command: "python".into(),
                    toolchain_config: json!({ "packageManager": "uv" }),
                    current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_venv_paths_if_no_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .extend_command(ExtendCommandInput {
                    command: "python".into(),
                    toolchain_config: json!({}),
                    current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
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
                .extend_command(ExtendCommandInput {
                    command: "python".into(),
                    toolchain_config: json!({}),
                    current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
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
                .extend_command(ExtendCommandInput {
                    command: "python".into(),
                    toolchain_config: json!({}),
                    current_dir: plugin
                        .plugin
                        .to_virtual_path(sandbox.path().join("sub/dir")),
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
                .extend_command(ExtendCommandInput {
                    command: "python".into(),
                    toolchain_config: json!({
                        "venvName": ".virtual-env"
                    }),
                    current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
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

    mod locate_dependencies_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_requirements_that_cant_be_parsed() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "pyproject.toml",
                r#"[project]
name = "root"
dependencies = ["not valid!!"]

[tool.uv.workspace]
members = ["packages/*"]
"#,
            );
            sandbox.create_file("packages/a/pyproject.toml", "[project]\nname = \"a\"\n");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: plugin
                        .plugin
                        .to_virtual_path(sandbox.path().join("packages/a")),
                    toolchain_config: json!({ "packageManager": "uv" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.root,
                Some(plugin.plugin.to_virtual_path(sandbox.path()))
            );
            assert_eq!(output.members, Some(vec!["packages/*".into()]));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_nothing_found() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert!(output.root.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_pm_not_configured() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::new(sandbox.path().join("package")),
                    toolchain_config: json!({
                        "packageManager": null
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert!(output.root.is_none());
        }

        mod pip {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn finds_package_without_lock() {
                let sandbox = create_moon_sandbox("locate");
                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(sandbox.path().join("package/nested")),
                        toolchain_config: json!({
                            "packageManager": "pip"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.members.is_none());
                assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/package"));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn finds_package_with_lock() {
                let sandbox = create_moon_sandbox("locate");
                sandbox.create_file("package/requirements.txt", "");

                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(sandbox.path().join("package/nested")),
                        toolchain_config: json!({
                            "packageManager": "pip"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.members.is_none());
                assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/package"));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn doesnt_support_workspaces() {
                let sandbox = create_moon_sandbox("locate");
                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(
                            sandbox.path().join("workspace/packages/a/nested"),
                        ),
                        toolchain_config: json!({
                            "packageManager": "pip"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.members.is_none());
                assert_eq!(
                    output.root.unwrap(),
                    VirtualPath::new("/workspace/workspace/packages/a")
                );
            }
        }

        mod poetry {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn finds_package_without_lock() {
                let sandbox = create_empty_moon_sandbox();
                sandbox.create_file("package/pyproject.toml", "");

                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(sandbox.path().join("package/nested")),
                        toolchain_config: json!({
                            "packageManager": "poetry"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.members.is_none());
                assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/package"));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn finds_package_with_lock() {
                let sandbox = create_empty_moon_sandbox();
                sandbox.create_file("package/pyproject.toml", "");
                sandbox.create_file("package/poetry.lock", "");

                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(sandbox.path().join("package/nested")),
                        toolchain_config: json!({
                            "packageManager": "poetry"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.members.is_none());
                assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/package"));
            }
        }

        mod uv {
            use super::*;

            #[tokio::test(flavor = "multi_thread")]
            async fn finds_package_without_lock() {
                let sandbox = create_moon_sandbox("locate");
                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(sandbox.path().join("package/nested")),
                        toolchain_config: json!({
                            "packageManager": "uv"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.members.is_none());
                assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/package"));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn finds_package_with_lock() {
                let sandbox = create_moon_sandbox("locate");
                sandbox.create_file("package/uv.lock", "");

                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(sandbox.path().join("package/nested")),
                        toolchain_config: json!({
                            "packageManager": "uv"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert!(output.members.is_none());
                assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/package"));
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn finds_workspace_without_lock() {
                let sandbox = create_moon_sandbox("locate");
                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(
                            sandbox.path().join("workspace/packages/a/nested"),
                        ),
                        toolchain_config: json!({
                            "packageManager": "uv"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(output.members.unwrap(), ["packages/*"]);
                assert_eq!(
                    output.root.unwrap(),
                    VirtualPath::new("/workspace/workspace")
                );
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn finds_workspace_with_lock() {
                let sandbox = create_moon_sandbox("locate");
                sandbox.create_file("workspace/uv.lock", "");

                let plugin = sandbox.create_toolchain("python").await;

                let output = plugin
                    .locate_dependencies_root(LocateDependenciesRootInput {
                        starting_dir: VirtualPath::new(
                            sandbox.path().join("workspace/packages/a/nested"),
                        ),
                        toolchain_config: json!({
                            "packageManager": "uv"
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(output.members.unwrap(), ["packages/*"]);
                assert_eq!(
                    output.root.unwrap(),
                    VirtualPath::new("/workspace/workspace")
                );
            }
        }
    }

    mod install_dependencies {
        use super::*;

        async fn get_install_args(
            sandbox: &MoonWasmSandbox,
            toolchain_config: serde_json::Value,
            production: bool,
            packages: Vec<String>,
        ) -> Option<Vec<String>> {
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config,
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    production,
                    packages,
                })
                .await;

            output
                .install_command
                .map(|cmd| [vec![cmd.command.command], cmd.command.args].concat())
        }

        fn strings(values: &[&str]) -> Vec<String> {
            values.iter().map(|value| value.to_string()).collect()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_syncs_all_packages_at_a_workspace_root() {
            for root in [
                "[project]\nname = \"root\"\n\n[tool.uv.workspace]\nmembers = [\"a\"]\n",
                // Virtual roots install every member anyways
                "[tool.uv.workspace]\nmembers = [\"a\"]\n",
            ] {
                let sandbox = create_empty_moon_sandbox();
                sandbox.create_file("pyproject.toml", root);

                assert_eq!(
                    get_install_args(&sandbox, json!({ "packageManager": "uv" }), false, vec![])
                        .await,
                    Some(strings(&["uv", "sync", "--all-packages", "--no-progress"]))
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_doesnt_sync_all_packages_outside_a_workspace() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("pyproject.toml", "[project]\nname = \"a\"\n");

            assert_eq!(
                get_install_args(&sandbox, json!({ "packageManager": "uv" }), false, vec![]).await,
                Some(strings(&["uv", "sync", "--no-progress"]))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_doesnt_sync_all_packages_with_package_args() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.create_file("pyproject.toml", "[tool.uv.workspace]\nmembers = [\"a\"]\n");
            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "syncArgs": ["--package=a"] }));

            assert_eq!(
                get_install_args(&sandbox, json!({ "packageManager": "uv" }), false, vec![]).await,
                Some(strings(&["uv", "sync", "--package=a", "--no-progress"]))
            );

            // Or for focused projects
            assert_eq!(
                get_install_args(
                    &sandbox,
                    json!({ "packageManager": "uv" }),
                    true,
                    vec!["b".into()]
                )
                .await,
                Some(strings(&[
                    "uv",
                    "sync",
                    "--package",
                    "b",
                    "--no-default-groups",
                    "--package=a",
                    "--no-progress"
                ]))
            );
        }

        async fn get_uv_sync_args(
            sandbox: &mut MoonWasmSandbox,
            ci: bool,
            sync_args: &[&str],
        ) -> Vec<String> {
            let sync_args = strings(sync_args);

            sandbox
                .host_funcs
                .mock_load_toolchain_config(move |_, _| json!({ "syncArgs": sync_args.clone() }));

            let plugin = sandbox
                .create_toolchain_with_config("python", |config| {
                    config.host_environment(HostEnvironment {
                        ci,
                        ..Default::default()
                    });
                })
                .await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "packageManager": "uv" }),
                    ..Default::default()
                })
                .await;

            output.install_command.unwrap().command.args
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_checks_the_lockfile_in_ci() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.create_file("uv.lock", "version = 1\n");

            assert_eq!(
                get_uv_sync_args(&mut sandbox, true, &[]).await,
                strings(&["sync", "--locked", "--no-progress"])
            );
            assert_eq!(
                get_uv_sync_args(&mut sandbox, false, &[]).await,
                strings(&["sync", "--no-progress"])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_doesnt_check_a_missing_lockfile_in_ci() {
            let mut sandbox = create_empty_moon_sandbox();

            assert_eq!(
                get_uv_sync_args(&mut sandbox, true, &[]).await,
                strings(&["sync", "--no-progress"])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_keeps_the_lock_mode_the_user_passed_in_ci() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.create_file("uv.lock", "version = 1\n");

            for arg in ["--frozen", "--locked"] {
                assert_eq!(
                    get_uv_sync_args(&mut sandbox, true, &[arg]).await,
                    strings(&["sync", arg, "--no-progress"])
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_keeps_the_lock_mode_from_env_vars_in_ci() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.create_file("uv.lock", "version = 1\n");

            for name in ["UV_FROZEN", "UV_LOCKED"] {
                // Each test runs in its own process with nextest
                unsafe { std::env::set_var(name, "1") };

                assert_eq!(
                    get_uv_sync_args(&mut sandbox, true, &[]).await,
                    strings(&["sync", "--no-progress"])
                );

                unsafe { std::env::remove_var(name) };
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_keeps_fallback_args_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "syncArgs": ["--all-extras"] }));

            assert_eq!(
                get_install_args(
                    &sandbox,
                    json!({ "packageManager": "uv", "version": "3.14.6" }),
                    false,
                    vec![]
                )
                .await,
                Some(strings(&[
                    "uv",
                    "sync",
                    "--all-extras",
                    "--no-managed-python",
                    "--no-python-downloads",
                    "--no-progress"
                ]))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uv_skips_fallback_args_the_user_passed() {
            for (args, expected) in [
                (
                    vec!["--no-progress", "--managed-python"],
                    vec!["uv", "sync", "--no-progress", "--managed-python"],
                ),
                (
                    vec!["--python-preference=managed"],
                    vec!["uv", "sync", "--python-preference=managed", "--no-progress"],
                ),
                (
                    vec!["--no-python-downloads"],
                    vec![
                        "uv",
                        "sync",
                        "--no-python-downloads",
                        "--no-managed-python",
                        "--no-progress",
                    ],
                ),
            ] {
                let mut sandbox = create_empty_moon_sandbox();
                sandbox
                    .host_funcs
                    .mock_load_toolchain_config(move |_, _| json!({ "syncArgs": args.clone() }));

                assert_eq!(
                    get_install_args(
                        &sandbox,
                        json!({ "packageManager": "uv", "version": "3.14.6" }),
                        false,
                        vec![]
                    )
                    .await,
                    Some(strings(&expected))
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uvpip_gets_fallback_args() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("requirements.txt", "");

            assert_eq!(
                get_install_args(
                    &sandbox,
                    json!({ "packageManager": "uv-pip", "version": "3.14.6" }),
                    false,
                    vec![]
                )
                .await,
                Some(strings(&[
                    "uv",
                    "pip",
                    "install",
                    "-r",
                    "requirements.txt",
                    "--no-managed-python",
                    "--no-python-downloads",
                    "--no-progress"
                ]))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn poetry_1_syncs_with_install() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                if id == "poetry" {
                    json!({ "version": "1.8.5" })
                } else {
                    json!({})
                }
            });

            assert_eq!(
                get_install_args(
                    &sandbox,
                    json!({ "packageManager": "poetry" }),
                    true,
                    vec![]
                )
                .await,
                Some(strings(&["poetry", "install", "--sync", "--only", "main"]))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn pip_installs_a_built_project() {
            for manifest in [
                "[build-system]\nrequires = [\"hatchling\"]\n\n[project]\nname = \"a\"\n",
                "[project]\nname = \"a\"\ndynamic = [\"dependencies\"]\n",
            ] {
                let sandbox = create_empty_moon_sandbox();
                sandbox.create_file("pyproject.toml", manifest);

                assert_eq!(
                    get_install_args(&sandbox, json!({ "packageManager": "pip" }), false, vec![])
                        .await,
                    Some(strings(&["python", "-m", "pip", "install", "-e", "."]))
                );
                assert_eq!(
                    get_install_args(
                        &sandbox,
                        json!({ "packageManager": "uv-pip" }),
                        false,
                        vec![]
                    )
                    .await,
                    Some(strings(&[
                        "uv",
                        "pip",
                        "install",
                        "-e",
                        ".",
                        "--no-progress"
                    ]))
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn pip_installs_only_dependencies_without_a_build_system() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "pyproject.toml",
                "[project]\nname = \"a\"\ndependencies = [\"six>=1\", \"idna\"]\n",
            );
            sandbox.create_file("constraints.txt", "");

            assert_eq!(
                get_install_args(&sandbox, json!({ "packageManager": "pip" }), false, vec![]).await,
                Some(strings(&[
                    "python",
                    "-m",
                    "pip",
                    "install",
                    "six>=1",
                    "idna",
                    "-c",
                    "constraints.txt"
                ]))
            );
            assert_eq!(
                get_install_args(
                    &sandbox,
                    json!({ "packageManager": "uv-pip" }),
                    false,
                    vec![]
                )
                .await,
                Some(strings(&[
                    "uv",
                    "pip",
                    "install",
                    "-r",
                    "pyproject.toml",
                    "-c",
                    "constraints.txt",
                    "--no-progress"
                ]))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn pip_does_nothing_without_dependencies() {
            for manifest in [
                "[project]\nname = \"a\"\n",
                // Only configures tools
                "[tool.ruff]\nline-length = 100\n",
            ] {
                let sandbox = create_empty_moon_sandbox();
                sandbox.create_file("pyproject.toml", manifest);
                sandbox.create_file("constraints.txt", "");

                for package_manager in ["pip", "uv-pip"] {
                    assert_eq!(
                        get_install_args(
                            &sandbox,
                            json!({ "packageManager": package_manager }),
                            false,
                            vec![]
                        )
                        .await,
                        None
                    );
                }
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn pip_prefers_requirements_over_the_project() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "pyproject.toml",
                "[build-system]\nrequires = [\"hatchling\"]\n\n[project]\nname = \"a\"\n",
            );
            sandbox.create_file("requirements.txt", "six\n");

            assert_eq!(
                get_install_args(&sandbox, json!({ "packageManager": "pip" }), false, vec![]).await,
                Some(strings(&[
                    "python",
                    "-m",
                    "pip",
                    "install",
                    "-r",
                    "requirements.txt"
                ]))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_pm() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.install_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_for_pip_without_requirements_or_project() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            // Simulate the virtual environment
            sandbox.create_file(".venv/bin/activate", "");

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.install_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_pip_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/bin/activate", "");
            sandbox.create_file("requirements.txt", "");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new(
                    "python",
                    [
                        "-m",
                        "pip",
                        "install",
                        "-r",
                        "requirements.txt",
                        "-a",
                        "b",
                        "--c",
                    ],
                )
                .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_pip_with_requirements() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("requirements.txt", "");
            sandbox.create_file(".venv/bin/activate", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new("python", ["-m", "pip", "install", "-r", "requirements.txt"])
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_pip_with_constraints() {
            let sandbox = create_empty_moon_sandbox();

            sandbox.create_file("requirements.txt", "");
            sandbox.create_file("constraints.txt", "");
            sandbox.create_file(".venv/bin/activate", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new(
                    "python",
                    [
                        "-m",
                        "pip",
                        "install",
                        "-r",
                        "requirements.txt",
                        "-c",
                        "constraints.txt",
                    ],
                )
                .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_poetry() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "poetry"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new("poetry", ["install"])
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_poetry_production() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "poetry"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    production: true,
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new("poetry", ["sync", "--only", "main"])
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_poetry_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "installArgs": ["-a", "b", "--c"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "poetry"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new("poetry", ["install", "-a", "b", "--c"])
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/bin/activate", "");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new("uv", ["sync", "--no-progress"])
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv_production() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/bin/activate", "");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    production: true,
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new("uv", ["sync", "--no-default-groups", "--no-progress"])
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv_with_version() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/bin/activate", "");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv",
                        "version": "1.2.3"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new(
                    "uv",
                    [
                        "sync",
                        "--no-managed-python",
                        "--no-python-downloads",
                        "--no-progress",
                    ],
                )
                .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/bin/activate", "");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "syncArgs": ["-a", "b", "--c"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new("uv", ["sync", "-a", "b", "--c", "--no-progress"])
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv_with_focused_projects() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;
            sandbox.create_file(".venv/bin/activate", "");

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    packages: vec!["foo".into(), "bar".into()],
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand {
                command: ExecCommandInput {
                    command: "uv".into(),
                    args: vec![
                        "sync".into(),
                        "--package".into(),
                        "foo".into(),
                        "--package".into(),
                        "bar".into(),
                        "--no-progress".into(),
                    ],
                    stream: true,
                    cwd: Some(plugin.plugin.to_virtual_path(sandbox.path())),
                    ..Default::default()
                },
                ..Default::default()
            };

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uvpip() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;
            sandbox.create_file(".venv/bin/activate", "");
            sandbox.create_file("requirements.txt", "");

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv-pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand {
                command: ExecCommandInput {
                    command: "uv".into(),
                    args: vec![
                        "pip".into(),
                        "install".into(),
                        "-r".into(),
                        "requirements.txt".into(),
                        "--no-progress".into(),
                    ],
                    cwd: Some(plugin.plugin.to_virtual_path(sandbox.path())),
                    stream: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uvpip_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();
            // Install args come from the pip toolchain (D1)
            sandbox.host_funcs.mock_load_toolchain_config(|id, _| {
                if id == "pip" {
                    json!({ "installArgs": ["-a", "b", "--c"] })
                } else {
                    json!({})
                }
            });
            let plugin = sandbox.create_toolchain("python").await;
            sandbox.create_file(".venv/bin/activate", "");
            sandbox.create_file("requirements.txt", "");

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "packageManager": "uv-pip" }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new(
                    "uv",
                    [
                        "pip",
                        "install",
                        "-r",
                        "requirements.txt",
                        "-a",
                        "b",
                        "--c",
                        "--no-progress",
                    ],
                )
                .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uvpip_with_requirements_and_constraints() {
            let sandbox = create_empty_moon_sandbox();
            // Add requirement & constraint files so the tool injects -r / -c
            sandbox.create_file("requirements.txt", "");
            sandbox.create_file("constraints.txt", "");
            sandbox.create_file(".venv/bin/activate", "");

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "packageManager": "uv-pip" }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            let actual = output.install_command.unwrap();
            let expected = ExecCommand::new(
                ExecCommandInput::new(
                    "uv",
                    [
                        "pip",
                        "install",
                        "-r",
                        "requirements.txt",
                        "-c",
                        "constraints.txt",
                        "--no-progress",
                    ],
                )
                .cwd(plugin.plugin.to_virtual_path(sandbox.path())),
            );

            assert_eq!(actual, expected);
        }
    }

    mod parse_lock {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_hashed_requirements_txt() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "requirements.txt",
                r#"# This file was autogenerated by uv via the following command:
idna==3.10 \
    --hash=sha256:12f65c9b470abda6dc35cf8e63cc574b1c52b11df2c86030af0ac09b01b13ea9
    # via requests
six==1.17.0  # via -r requirements.in
not valid!!
"#,
            );

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: plugin
                        .plugin
                        .to_virtual_path(sandbox.path().join("requirements.txt")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.dependencies.keys().collect::<Vec<_>>(),
                ["idna", "six"]
            );
            assert_eq!(
                output.dependencies["idna"][0].hash.as_deref(),
                Some("12f65c9b470abda6dc35cf8e63cc574b1c52b11df2c86030af0ac09b01b13ea9")
            );
            assert_eq!(output.dependencies["six"][0].hash, None);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_pylock_toml() {
            let sandbox = create_moon_sandbox("lockfiles");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::new(sandbox.path().join("pylock.toml")),
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
        async fn parses_poetry_lock() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "poetry.lock",
                r#"
[[package]]
name = "attrs"
version = "25.1.0"
description = "Classes Without Boilerplate"
optional = false
python-versions = ">=3.8"
files = [
    {file = "attrs-25.1.0-py3-none-any.whl", hash = "sha256:c75a69e28a550a7e93789579c22aa26b0f5b83b75dc4e08fe092980051e1090a"},
]

[[package]]
name = "cattrs"
version = "24.1.2"
description = "Composable complex class support"
optional = false
python-versions = ">=3.8"
files = []
"#,
            );

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::new(sandbox.path().join("poetry.lock")),
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
                            version: Some(VersionSpec::parse("24.1.2").unwrap()),
                            ..Default::default()
                        }]
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_requirements_txt() {
            let sandbox = create_moon_sandbox("lockfiles");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::new(sandbox.path().join("requirements.txt")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "docopt".into(),
                        vec![LockDependency {
                            version: Some(VersionSpec::parse("0.6.1").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    ("pytest".into(), vec![LockDependency::default()]),
                    ("pytest-cov".into(), vec![LockDependency::default()]),
                    (
                        "requests".into(),
                        vec![LockDependency {
                            meta: Some("security".into()),
                            req: Some(UnresolvedVersionSpec::parse("==2.8.*, >=2.8.1").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    ("urllib3".into(), vec![LockDependency::default()]),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_uv_lock() {
            let sandbox = create_moon_sandbox("lockfiles");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::new(sandbox.path().join("uv.lock")),
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

    mod parse_manifest {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_requirements_that_cant_be_parsed() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "pyproject.toml",
                "[project]\nname = \"a\"\nreadme = 1\ndependencies = [\"six\", \"not valid!!\"]\n",
            );

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: plugin
                        .plugin
                        .to_virtual_path(sandbox.path().join("pyproject.toml")),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.dependencies.keys().collect::<Vec<_>>(), ["six"]);
        }

        fn versions(deps: &BTreeMap<String, ManifestDependency>) -> Vec<(&str, String)> {
            deps.iter()
                .map(|(name, dep)| (name.as_str(), dep.get_version().unwrap().to_string()))
                .collect()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reports_every_table_by_scope() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "pyproject.toml",
                r#"
[build-system]
requires = ["hatchling>=1.26"]

[project]
name = "a"
dependencies = ["six", "idna~=3.10", "certifi==2025.1.31", "Typing_Extensions>=4.12"]

[project.optional-dependencies]
http = ["requests>=2.32.0", "six>=1.0"]

[dependency-groups]
dev = ["pytest>=8", { include-group = "lint" }]
lint = ["ruff"]

[tool.uv]
dev-dependencies = ["mypy"]

[tool.poetry.dependencies]
python = "^3.12"
Flask_Cors = "^6.0"
local = { path = "../local", develop = true }
multi = [{ version = "1", python = "<3.9" }, { version = "2", python = ">=3.9" }]

[tool.poetry.group.test.dependencies]
coverage = { version = "~7.6", extras = ["toml"] }

[tool.poetry.dev-dependencies]
black = "*"
"#,
            );

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::new(sandbox.path().join("pyproject.toml")),
                    ..Default::default()
                })
                .await;

            // Unversioned, `~=` and multiple constraints match any locked version,
            // and a dependency keeps the version from its first table
            assert_eq!(
                versions(&output.dependencies),
                [
                    ("certifi", "2025.1.31".into()),
                    ("flask-cors", "^6.0".into()),
                    ("idna", "*".into()),
                    ("local", "*".into()),
                    ("multi", "*".into()),
                    ("requests", ">=2.32.0".into()),
                    ("six", "*".into()),
                    ("typing-extensions", ">=4.12".into()),
                ]
            );
            assert_eq!(
                versions(&output.dev_dependencies),
                [
                    ("black", "*".into()),
                    ("coverage", "~7.6".into()),
                    ("mypy", "*".into()),
                    ("pytest", ">=8".into()),
                    ("ruff", "*".into()),
                ]
            );
            assert_eq!(
                versions(&output.build_dependencies),
                [("hatchling", ">=1.26".into())]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reports_poetry_1_projects() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "pyproject.toml",
                "[tool.poetry]\nname = \"a\"\n\n[tool.poetry.dependencies]\npython = \"^3.8\"\nrequests = \"^2.31\"\n",
            );

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::new(sandbox.path().join("pyproject.toml")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                versions(&output.dependencies),
                [("requests", "^2.31".into())]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_pyproject_toml() {
            let sandbox = create_moon_sandbox("manifests");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::new(sandbox.path().join("pyproject.toml")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "django".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse(">2.1").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "gidgethub".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            features: vec!["httpx".into()],
                            version: Some(UnresolvedVersionSpec::parse(">4.0.0").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "httpx".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("6.7.8").unwrap()),
                            ..Default::default()
                        })
                    ),
                    // From optional dependencies
                    (
                        "mysqlclient".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse(">=1.3, <2.0").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "psycopg2".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse(">=2.9, <3.0").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "requests".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse(">=2.13.0").unwrap()),
                            ..Default::default()
                        })
                    ),
                ])
            );
            assert_eq!(
                output.build_dependencies,
                BTreeMap::from_iter([(
                    "uv-build".into(),
                    ManifestDependency::Config(ManifestDependencyConfig {
                        version: Some(UnresolvedVersionSpec::parse(">=0.9.18, <0.10.0").unwrap()),
                        ..Default::default()
                    })
                )])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_requirements_in() {
            let sandbox = create_moon_sandbox("manifests");
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::new(sandbox.path().join("requirements.in")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "pytest".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("*").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "pytest-cov".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("*").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "docopt".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("0.6.1").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "requests".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            features: vec!["security".into()],
                            version: Some(
                                UnresolvedVersionSpec::parse("==2.8.*, >=2.8.1").unwrap()
                            ),
                            ..Default::default()
                        })
                    ),
                    (
                        "urllib3".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            url: Some(
                                "https://github.com/urllib3/urllib3/archive/refs/tags/1.26.8.zip"
                                    .into()
                            ),
                            version: Some(UnresolvedVersionSpec::parse("*").unwrap()),
                            ..Default::default()
                        })
                    ),
                ])
            );
        }
    }

    mod setup_environment {
        use super::*;

        async fn get_setup_commands(
            sandbox: &MoonWasmSandbox,
            toolchain_config: serde_json::Value,
        ) -> Vec<Vec<String>> {
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config,
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            output
                .commands
                .into_iter()
                .map(|cmd| [vec![cmd.command.command], cmd.command.args].concat())
                .collect()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn recreates_the_venv_when_its_python_changed() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/pyvenv.cfg", "home = /python/bin\nversion = 3.14.6\n");
            sandbox.create_file(".venv/bin/python", "");

            let commands = get_setup_commands(
                &sandbox,
                json!({ "packageManager": "uv", "version": "3.14.7" }),
            )
            .await;

            assert_eq!(
                commands,
                [[
                    "uv",
                    "venv",
                    ".venv",
                    "--no-managed-python",
                    "--no-python-downloads",
                    "--no-progress"
                ]]
            );
            assert!(!sandbox.path().join(".venv").exists());

            sandbox.create_file(".venv/pyvenv.cfg", "version_info = 3.14.6\n");

            let commands = get_setup_commands(
                &sandbox,
                json!({ "packageManager": "pip", "version": "3.13.0" }),
            )
            .await;

            assert_eq!(commands, [["python", "-m", "venv", ".venv"]]);
            assert!(!sandbox.path().join(".venv").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_the_venv_when_its_python_matches() {
            for version in [json!("3.14.6"), json!("3.14"), json!("latest"), json!(null)] {
                let sandbox = create_empty_moon_sandbox();
                sandbox.create_file(".venv/pyvenv.cfg", "version_info = 3.14.6\n");

                let commands = get_setup_commands(
                    &sandbox,
                    json!({ "packageManager": "uv", "version": version }),
                )
                .await;

                assert!(commands.is_empty());
                assert!(sandbox.path().join(".venv/pyvenv.cfg").exists());
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn keeps_the_venv_without_a_pyvenv_cfg() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/bin/python", "");

            let commands = get_setup_commands(
                &sandbox,
                json!({ "packageManager": "uv", "version": "3.14.7" }),
            )
            .await;

            assert!(commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_package_manager() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": null
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_customize_dir_name() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv",
                        "venvName": ".virtual-env"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("uv", ["venv", ".virtual-env", "--no-progress"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_pip() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("python", ["-m", "venv", ".venv"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_pip_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "venvArgs": ["--clear"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("python", ["-m", "venv", ".venv", "--clear"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_poetry() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "poetry"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("python", ["-m", "venv", ".venv"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_poetry_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "venvArgs": ["--clear"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "poetry"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("python", ["-m", "venv", ".venv", "--clear"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("uv", ["venv", ".venv", "--no-progress"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv_with_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv",
                        "version": "1.2.3"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new(
                        "uv",
                        [
                            "venv",
                            ".venv",
                            "--no-managed-python",
                            "--no-python-downloads",
                            "--no-progress"
                        ]
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uv_with_custom_args() {
            let mut sandbox = create_empty_moon_sandbox();

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "venvArgs": ["--clear"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("uv", ["venv", ".venv", "--clear", "--no-progress"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_proto_python_when_version_defined() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv",
                        "version": "3.12.0"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new(
                        "uv",
                        [
                            "venv",
                            ".venv",
                            "--no-managed-python",
                            "--no-python-downloads",
                            "--no-progress",
                        ]
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_uvpip() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "packageManager": "uv-pip" }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;
            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("uv", ["venv", ".venv", "--no-progress"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_proto_python_when_version_defined_uvpip() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "packageManager": "uv-pip", "version": "3.12.0" }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;
            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new(
                        "uv",
                        [
                            "venv",
                            ".venv",
                            "--no-managed-python",
                            "--no-python-downloads",
                            "--no-progress",
                        ]
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_setup_if_venv_already_exists() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/pyvenv.cfg", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_setup_if_venv_already_exists_with_pip() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/pyvenv.cfg", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_setup_if_venv_already_exists_with_uvpip() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/pyvenv.cfg", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv-pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_setup_if_custom_venv_already_exists() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".virtual-env/pyvenv.cfg", "");

            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv",
                        "venvName": ".virtual-env"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_not_skip_setup_if_clear_arg_is_passed() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/pyvenv.cfg", "");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "venvArgs": ["--clear"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "uv"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("uv", ["venv", ".venv", "--clear", "--no-progress"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_not_skip_setup_if_clear_arg_is_passed_with_pip() {
            let mut sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".venv/pyvenv.cfg", "");

            sandbox
                .host_funcs
                .mock_load_toolchain_config(|_, _| json!({ "venvArgs": ["--clear"]}));

            let plugin = sandbox.create_toolchain("python").await;
            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "packageManager": "pip"
                    }),
                    project: Some(ProjectFragment {
                        id: Id::raw("workspace"),
                        source: ".".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new("python", ["-m", "venv", ".venv", "--clear"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }
    }
}
