use moon_config::DependencyScope;
use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

mod rust_toolchain_tier2 {
    use super::*;

    mod extend_project_graph {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn loads_for_all_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("rust").await;

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
                            alias: Some("a".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        "b".into(),
                        ExtendProjectOutput {
                            alias: Some("b".into()),
                            ..Default::default()
                        }
                    ),
                    (
                        "c".into(),
                        ExtendProjectOutput {
                            alias: Some("c".into()),
                            dependencies: vec![
                                ProjectDependency {
                                    id: "a".into(),
                                    scope: DependencyScope::Production,
                                    via: Some("crate a".into()),
                                },
                                ProjectDependency {
                                    id: "b".into(),
                                    scope: DependencyScope::Development,
                                    via: Some("crate b".into()),
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
                    PathBuf::from("/workspace/a/Cargo.toml"),
                    PathBuf::from("/workspace/b/Cargo.toml"),
                    PathBuf::from("/workspace/c/Cargo.toml"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_projects_not_in_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("rust").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert("a".into(), "a".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects,
                BTreeMap::from_iter([(
                    "a".into(),
                    ExtendProjectOutput {
                        alias: Some("a".into()),
                        ..Default::default()
                    }
                ),])
            );

            assert_eq!(
                output.input_files,
                [PathBuf::from("/workspace/a/Cargo.toml")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_projects_without_a_manifest() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("rust").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert("no-manifest".into(), "no-manifest".into());

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
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(
                        sandbox.path().join(".home/.cargo-custom"),
                    )),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".home/.cargo-custom")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fallsback_to_cargo_dir_when_no_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            unsafe {
                env::remove_var("CARGO_INSTALL_ROOT");
                env::remove_var("CARGO_HOME");
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
                [sandbox.path().join(".home").join(".cargo").join("bin")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prefixes_with_cargo_if_a_cargo_bin() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".home/.cargo/bin/cargo-nextest", "");
            sandbox.create_file(".home/.cargo/bin/cargo-nextest.exe", "");

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "nextest".into(),
                    args: vec!["run".into()],
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.command.unwrap(), "cargo");
            assert_eq!(
                output.args.unwrap(),
                Extend::Prepend(vec!["nextest".into()])
            );

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "cargo-nextest".into(),
                    args: vec!["run".into()],
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.command.unwrap(), "cargo");
            assert_eq!(
                output.args.unwrap(),
                Extend::Prepend(vec!["nextest".into()])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_prefix_with_cargo_if_a_global_bin() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".home/.cargo/bin/nextest", "");
            sandbox.create_file(".home/.cargo/bin/nextest.exe", "");

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "nextest".into(),
                    args: vec!["run".into()],
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_prefix_if_already_rust_or_cargo() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".home/.cargo/bin/cargo-nextest", "");
            sandbox.create_file(".home/.cargo/bin/cargo-nextest.exe", "");

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "cargo".into(),
                    args: vec!["build".into()],
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "rustc".into(),
                    globals_dir: Some(VirtualPath::Real(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
        }
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "unknown".into(),
                    globals_dir: Some(VirtualPath::Real(
                        sandbox.path().join(".home/.cargo-custom"),
                    )),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
            assert!(output.env_remove.is_empty());
            assert_eq!(output.paths, [sandbox.path().join(".home/.cargo-custom")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fallsback_to_cargo_dir_when_no_globals_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            unsafe {
                env::remove_var("CARGO_INSTALL_ROOT");
                env::remove_var("CARGO_HOME");
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
                [sandbox.path().join(".home").join(".cargo").join("bin")]
            );
        }
    }

    mod locate_dependencies_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_nothing_found() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("rust").await;

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
            let plugin = sandbox.create_toolchain("rust").await;

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
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("package-with-lock/nested"),
                    ),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(
                output.root.unwrap(),
                PathBuf::from("/workspace/package-with-lock")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_without_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace/crates/a/nested"),
                    ),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["crates/*"]);
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/workspace"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(
                        sandbox.path().join("workspace-with-lock/crates/a/nested"),
                    ),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["crates/*"]);
            assert_eq!(
                output.root.unwrap(),
                PathBuf::from("/workspace/workspace-with-lock")
            );
        }
    }

    mod install_dependencies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_lock_exists() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert!(output.install_command.is_none());
            assert!(output.dedupe_command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_install_command_if_no_lock() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("cargo", ["generate-lockfile"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
            assert!(output.dedupe_command.is_none());
        }
    }

    mod parse_lock {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn parses() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("Cargo.lock")),
                    ..Default::default()
                })
                .await;

            assert!(output.packages.is_empty());
            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "base64".into(),
                        vec![
                            LockDependency {
                                hash: Some(
                                    "9d297deb1925b89f2ccc13d7635fa0714f12c87adce1c75356b39ca9b7178567"
                                        .into()
                                ),
                                version: Some(VersionSpec::parse("0.21.7").unwrap()),
                                ..Default::default()
                            },
                            LockDependency {
                                hash: Some(
                                    "72b3254f16251a8381aa12e40e3c4d2f0199f8c6508fbecb9d91f575e0fbb8c6"
                                        .into()
                                ),
                                version: Some(VersionSpec::parse("0.22.1").unwrap()),
                                ..Default::default()
                            }
                        ]
                    ),
                    (
                        "moon_pdk_api".into(),
                        vec![LockDependency {
                            version: Some(VersionSpec::parse("0.1.6").unwrap()),
                            ..Default::default()
                        }]
                    )
                ])
            );
        }
    }

    mod parse_manifest {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_workspace() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::Real(sandbox.path().join("Cargo.toml")),
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
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("4.5.6").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "c".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            features: vec!["on".into()],
                            version: Some(UnresolvedVersionSpec::parse("7.8.9").unwrap()),
                            ..Default::default()
                        })
                    )
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::Real(sandbox.path().join("package/Cargo.toml")),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.version.unwrap(), Version::parse("1.0.0").unwrap());
            assert!(output.publishable);
            assert!(output.peer_dependencies.is_empty());

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    (
                        "a".into(),
                        ManifestDependency::Version(UnresolvedVersionSpec::parse("1.2.3").unwrap())
                    ),
                    (
                        "e".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            features: vec!["on".into()],
                            version: Some(UnresolvedVersionSpec::parse("7.8.9").unwrap()),
                            ..Default::default()
                        })
                    )
                ])
            );

            assert_eq!(
                output.dev_dependencies,
                BTreeMap::from_iter([
                    (
                        "b".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("4.5.6").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "f".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            path: Some("../other".into()),
                            ..Default::default()
                        })
                    )
                ])
            );

            assert_eq!(
                output.build_dependencies,
                BTreeMap::from_iter([
                    ("c".into(), ManifestDependency::Inherited(true)),
                    (
                        "d".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            inherited: true,
                            features: vec!["off".into()],
                            ..Default::default()
                        })
                    )
                ])
            );
        }
    }
}
