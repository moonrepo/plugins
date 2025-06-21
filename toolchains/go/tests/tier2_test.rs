use moon_config::DependencyScope;
use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

mod go_toolchain_tier2 {
    use super::*;

    mod extend_project_graph {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn loads_for_all_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("go").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert("a".into(), "a".into());
            input.project_sources.insert("b".into(), "b".into());
            input.project_sources.insert("c".into(), "c".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        ExtendProjectOutput {
                            alias: Some("example.com/org/a".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        "b".into(),
                        ExtendProjectOutput {
                            alias: Some("example.com/org/b".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        "c".into(),
                        ExtendProjectOutput {
                            alias: Some("example.com/org/c".into()),
                            dependencies: vec![
                                ProjectDependency {
                                    id: "example.com/org/a".into(),
                                    scope: DependencyScope::Production,
                                    via: Some("module example.com/org/a".into()),
                                },
                                ProjectDependency {
                                    id: "example.com/org/b".into(),
                                    scope: DependencyScope::Production,
                                    via: Some("module example.com/org/b".into()),
                                }
                            ],
                            ..Default::default()
                        }
                    ),
                ])
            );

            assert_eq!(
                output.input_files,
                [
                    PathBuf::from("/workspace/a/go.mod"),
                    PathBuf::from("/workspace/b/go.mod"),
                    PathBuf::from("/workspace/c/go.mod"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_projects_not_in_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("go").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert("a".into(), "a".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([(
                    "a".into(),
                    ExtendProjectOutput {
                        alias: Some("example.com/org/a".into()),
                        ..Default::default()
                    }
                ),])
            );

            assert_eq!(output.input_files, [PathBuf::from("/workspace/a/go.mod")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_projects_without_a_manifest() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("go").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert("no-mod".into(), "no-mod".into());

            let output = plugin.extend_project_graph(input).await;

            assert!(output.extended_projects.is_empty());
            assert!(output.input_files.is_empty());
        }
    }

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.go-bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".home/.go-bin")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fallsback_to_go_dir_when_no_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            unsafe {
                env::remove_var("GOBIN");
                env::remove_var("GOPATH");
            }

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(
                output.paths,
                [sandbox.path().join(".home").join("go").join("bin")]
            );
        }
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.go-bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".home/.go-bin")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fallsback_to_go_dir_when_no_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            unsafe {
                env::remove_var("GOBIN");
                env::remove_var("GOPATH");
            }

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(
                output.paths,
                [sandbox.path().join(".home").join("go").join("bin")]
            );
        }
    }

    mod locate_dependencies_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_nothing_found() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert!(output.root.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_without_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package/nested")),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_with_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("package-with-sum/nested")),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(
                output.root.unwrap(),
                PathBuf::from("/workspace/package-with-sum")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/modules/a/nested"),
                    ),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["modules/a"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }
    }

    mod install_dependencies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_files() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.install_command.is_none());
            assert!(output.dedupe_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_install_command_for_go_work_if_enabled() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.work", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "workspaces": true
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("go", ["work", "sync"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
            assert!(output.dedupe_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_set_install_command_for_go_work_if_disabled() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.work", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "workspaces": false
                    }),
                    ..Default::default()
                })
                .await;

            assert!(output.install_command.is_none());
            assert!(output.dedupe_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn workspaces_take_precedence() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.work", "");
            sandbox.create_file("go.mod", "");
            sandbox.create_file("go.sum", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "tidyOnChange": true,
                        "workspaces": true
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("go", ["work", "sync"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
            assert!(output.dedupe_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_install_command_for_go_mod() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.mod", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("go", ["mod", "download"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
            assert!(output.dedupe_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_dedupe_command_for_go_sum_if_enabled() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.mod", "");
            sandbox.create_file("go.sum", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "tidyOnChange": true
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("go", ["mod", "download"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
            assert_eq!(
                output.dedupe_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("go", ["mod", "tidy"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_set_dedupe_command_for_go_sum_if_disabled() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.mod", "");
            sandbox.create_file("go.sum", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "tidyOnChange": false
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("go", ["mod", "download"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
            assert!(output.dedupe_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_set_dedupe_command_for_go_sum_if_no_file() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.mod", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "tidyOnChange": true
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("go", ["mod", "download"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
            assert!(output.dedupe_command.is_none());
        }
    }

    mod parse_lock {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_go_sum() {
            let sandbox = create_moon_sandbox("sum-files");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("basic.sum")),
                    ..Default::default()
                })
                .await;

            assert!(output.packages.is_empty());
            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "github.com/atotto/clipboard".into(),
                        vec![LockDependency {
                            hash: Some("EH0zSVneZPSuFR11BlR9YppQTVDbh5+16AmcJi4g1z4=".into()),
                            version: Some(VersionSpec::parse("0.1.4").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "github.com/charmbracelet/bubbles".into(),
                        vec![LockDependency {
                            hash: Some("9TdC97SdRVg/1aaXNVWfFH3nnLAwOXr8Fn6u6mfQdFs=".into()),
                            version: Some(VersionSpec::parse("0.21.0").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "github.com/charmbracelet/bubbletea".into(),
                        vec![LockDependency {
                            hash: Some("JAMNLTbqMOhSwoELIr0qyP4VidFq72/6E9j7HHmRKQc=".into()),
                            version: Some(VersionSpec::parse("1.3.5").unwrap()),
                            ..Default::default()
                        }]
                    ),
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_go_work_sum() {
            let sandbox = create_moon_sandbox("work-files");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("basic.work.sum")),
                    ..Default::default()
                })
                .await;

            assert!(output.packages.is_empty());
            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "github.com/atotto/clipboard".into(),
                        vec![LockDependency {
                            hash: Some("EH0zSVneZPSuFR11BlR9YppQTVDbh5+16AmcJi4g1z4=".into()),
                            version: Some(VersionSpec::parse("0.1.4").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "github.com/charmbracelet/bubbles".into(),
                        vec![LockDependency {
                            hash: Some("9TdC97SdRVg/1aaXNVWfFH3nnLAwOXr8Fn6u6mfQdFs=".into()),
                            version: Some(VersionSpec::parse("0.21.0").unwrap()),
                            ..Default::default()
                        }]
                    ),
                    (
                        "github.com/charmbracelet/bubbletea".into(),
                        vec![LockDependency {
                            hash: Some("JAMNLTbqMOhSwoELIr0qyP4VidFq72/6E9j7HHmRKQc=".into()),
                            version: Some(VersionSpec::parse("1.3.5").unwrap()),
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
        async fn parses_package() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::Real(sandbox.path().join("c/go.mod")),
                    ..Default::default()
                })
                .await;

            assert!(output.version.is_none());
            assert!(!output.publishable);
            assert!(output.peer_dependencies.is_empty());
            assert!(output.dev_dependencies.is_empty());
            assert!(output.build_dependencies.is_empty());

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "example.com/org/a".into(),
                        ManifestDependency::Version(UnresolvedVersionSpec::parse("1.2.3").unwrap())
                    ),
                    (
                        "example.com/org/b".into(),
                        ManifestDependency::Version(UnresolvedVersionSpec::parse("4.5.6").unwrap())
                    )
                ])
            );
        }
    }

    mod bins {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_add_command_if_empty() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

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
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "bins": [
                            "golang.org/x/tools/gopls",
                            {
                                "bin": "github.com/revel/cmd/revel"
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
                        "go",
                        [
                            "install",
                            "-v",
                            "golang.org/x/tools/gopls",
                            "github.com/revel/cmd/revel"
                        ],
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
                .cache("go-bins-latest")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn separates_commands_by_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "bins": [
                            "golang.org/x/tools/gopls@1",
                            {
                                "bin": "github.com/revel/cmd/revel@2"
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
                            "go",
                            ["install", "-v", "golang.org/x/tools/gopls@1"],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("go-bins-1"),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "go",
                            ["install", "-v", "github.com/revel/cmd/revel@2"],
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                    .cache("go-bins-2")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_local_bins_when_ci() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.enable_logging();

            let plugin = sandbox
                .create_toolchain_with_config("go", |config| {
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
                                "bin": "github.com/revel/cmd/revel",
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
