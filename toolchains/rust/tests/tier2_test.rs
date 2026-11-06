use moon_common::Id;
use moon_config::DependencyScope;
use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::collections::BTreeMap;
use std::env;

mod rust_toolchain_tier2 {
    use super::*;

    mod extend_project_graph {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn loads_for_all_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("rust").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("a"), "a".into());
            input.project_sources.insert(Id::raw("b"), "b".into());
            input.project_sources.insert(Id::raw("c"), "c".into());

            let output = plugin.extend_project_graph(input).await;

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
                                    via: Some("crate a".into()),
                                },
                                ProjectDependency {
                                    id: Id::raw("b"),
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
                    VirtualPath::new("/workspace/Cargo.toml"),
                    VirtualPath::new("/workspace/a/Cargo.toml"),
                    VirtualPath::new("/workspace/b/Cargo.toml"),
                    VirtualPath::new("/workspace/c/Cargo.toml"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn links_inherited_renamed_and_target_deps() {
            let sandbox = create_moon_sandbox("projects-inherited");
            let plugin = sandbox.create_toolchain("rust").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("app"), "app".into());
            input.project_sources.insert(Id::raw("core"), "core".into());
            input
                .project_sources
                .insert(Id::raw("real"), "real-name".into());
            input
                .project_sources
                .insert(Id::raw("unix"), "unix-only".into());
            input
                .project_sources
                .insert(Id::raw("versioned"), "versioned".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects.get("app").unwrap().dependencies,
                [
                    ProjectDependency {
                        id: Id::raw("core"),
                        scope: DependencyScope::Production,
                        via: Some("crate core".into()),
                    },
                    ProjectDependency {
                        id: Id::raw("real"),
                        scope: DependencyScope::Production,
                        via: Some("crate real-name".into()),
                    },
                    ProjectDependency {
                        id: Id::raw("unix"),
                        scope: DependencyScope::Production,
                        via: Some("crate unix-only".into()),
                    },
                    ProjectDependency {
                        id: Id::raw("core"),
                        scope: DependencyScope::Development,
                        via: Some("crate core".into()),
                    },
                ]
            );

            assert_eq!(
                output.input_files,
                [
                    VirtualPath::new("/workspace/Cargo.toml"),
                    VirtualPath::new("/workspace/app/Cargo.toml"),
                    VirtualPath::new("/workspace/core/Cargo.toml"),
                    VirtualPath::new("/workspace/real-name/Cargo.toml"),
                    VirtualPath::new("/workspace/unix-only/Cargo.toml"),
                    VirtualPath::new("/workspace/versioned/Cargo.toml"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_projects_not_in_sources() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("rust").await;

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
                )])
            );

            assert_eq!(
                output.input_files,
                [
                    VirtualPath::new("/workspace/Cargo.toml"),
                    VirtualPath::new("/workspace/a/Cargo.toml")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_projects_without_a_manifest() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_toolchain("rust").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert(Id::raw("no-manifest"), "no-manifest".into());

            let output = plugin.extend_project_graph(input).await;

            assert!(output.extended_projects.is_empty());
            assert!(output.input_files.is_empty());
        }
    }

    mod extend_task_command {
        use super::*;

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
        async fn includes_cargo_home_with_install_root() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            unsafe {
                env::set_var("CARGO_INSTALL_ROOT", sandbox.path().join("install"));
                env::remove_var("CARGO_HOME");
            }

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "unknown".into(),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.paths,
                [
                    sandbox.path().join("install").join("bin"),
                    sandbox.path().join(".home").join(".cargo").join("bin")
                ]
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
                    globals_dir: Some(VirtualPath::new(sandbox.path().join(".home/.cargo/bin"))),
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
                    globals_dir: Some(VirtualPath::new(sandbox.path().join(".home/.cargo/bin"))),
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
                    globals_dir: Some(VirtualPath::new(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_prefix_with_cargo_if_command_exists() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".home/.cargo/bin/cargo-git", "");
            sandbox.create_file(".home/.cargo/bin/cargo-git.exe", "");

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "git".into(),
                    args: vec!["status".into()],
                    globals_dir: Some(VirtualPath::new(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());

            // Explicit `cargo-*` commands are still prefixed
            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "cargo-git".into(),
                    globals_dir: Some(VirtualPath::new(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.command.unwrap(), "cargo");
            assert_eq!(output.args.unwrap(), Extend::Prepend(vec!["git".into()]));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn pins_toolchain_for_versions_and_channels() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            unsafe {
                env::remove_var("RUSTUP_TOOLCHAIN");
            }

            for (version, channel) in [
                ("1.80.0", Some("1.80.0")),
                ("stable", Some("stable")),
                ("nightly-2024-01-01", Some("nightly-2024-01-01")),
                ("1.80", None),
            ] {
                let output = plugin
                    .extend_task_command(ExtendTaskCommandInput {
                        command: "cargo".into(),
                        toolchain_config: json!({ "version": version }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(
                    output
                        .env
                        .get("RUSTUP_TOOLCHAIN")
                        .map(|value| value.as_str()),
                    channel
                );
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_pin_toolchain_if_set_on_host() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            unsafe {
                env::set_var("RUSTUP_TOOLCHAIN", "nightly");
            }

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "cargo".into(),
                    toolchain_config: json!({ "version": "1.80.0" }),
                    ..Default::default()
                })
                .await;

            assert!(output.env.is_empty());
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
                    globals_dir: Some(VirtualPath::new(sandbox.path().join(".home/.cargo/bin"))),
                    ..Default::default()
                })
                .await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "rustc".into(),
                    globals_dir: Some(VirtualPath::new(sandbox.path().join(".home/.cargo/bin"))),
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

        #[tokio::test(flavor = "multi_thread")]
        async fn pins_toolchain() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            unsafe {
                env::remove_var("RUSTUP_TOOLCHAIN");
            }

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "cargo build && cargo test".into(),
                    toolchain_config: json!({ "version": "1.80.0" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output
                    .env
                    .get("RUSTUP_TOOLCHAIN")
                    .map(|value| value.as_str()),
                Some("1.80.0")
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
                    starting_dir: VirtualPath::new(sandbox.path()),
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
                    starting_dir: VirtualPath::new(sandbox.path().join("package/nested")),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/package"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_package_with_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::new(sandbox.path().join("package-with-lock/nested")),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert_eq!(
                output.root.unwrap(),
                VirtualPath::new("/workspace/package-with-lock")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_without_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::new(
                        sandbox.path().join("workspace/crates/a/nested"),
                    ),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["crates/*"]);
            assert_eq!(
                output.root.unwrap(),
                VirtualPath::new("/workspace/workspace")
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_workspace_with_lock() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::new(
                        sandbox.path().join("workspace-with-lock/crates/a/nested"),
                    ),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.members.unwrap(), ["crates/*"]);
            assert_eq!(
                output.root.unwrap(),
                VirtualPath::new("/workspace/workspace-with-lock")
            );
        }
    }

    mod locate_dependencies_roots {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_each_dir_in_order() {
            let sandbox = create_moon_sandbox("locate");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .locate_dependencies_roots(LocateDependenciesRootsInput {
                    starting_dirs: vec![
                        VirtualPath::new(
                            sandbox.path().join("workspace-with-lock/crates/a/nested"),
                        ),
                        VirtualPath::new(sandbox.path()),
                        VirtualPath::new(sandbox.path().join("package/nested")),
                    ],
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.roots,
                vec![
                    LocateDependenciesRootOutput {
                        members: Some(vec!["crates/*".into()]),
                        root: Some(VirtualPath::new("/workspace/workspace-with-lock")),
                    },
                    LocateDependenciesRootOutput::default(),
                    LocateDependenciesRootOutput {
                        members: None,
                        root: Some(VirtualPath::new("/workspace/package")),
                    },
                ]
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
                    root: VirtualPath::new(sandbox.path()),
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
                    root: VirtualPath::new(sandbox.path()),
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
                    path: VirtualPath::new(sandbox.path().join("Cargo.lock")),
                    ..Default::default()
                })
                .await;

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
                    path: VirtualPath::new(sandbox.path().join("Cargo.toml")),
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
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("^1.2.3").unwrap()
                        )
                    ),
                    (
                        "b".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            version: Some(UnresolvedVersionSpec::parse("^4.5.6").unwrap()),
                            ..Default::default()
                        })
                    ),
                    (
                        "c".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            features: vec!["on".into()],
                            version: Some(UnresolvedVersionSpec::parse("^7.8.9").unwrap()),
                            ..Default::default()
                        })
                    )
                ])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_target_deps_and_caret_requirements() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "Cargo.toml",
                r#"[package]
name = "pkg"
version = "1.0.0"

[dependencies]
both = "1"
exact = "=1.2.3"
tokio = "1.40"

[target.'cfg(unix)'.dependencies]
both = { version = "2", features = ["x"] }
libc = "0.2"

[target.'cfg(windows)'.dev-dependencies]
winapi = "0.3.9"
"#,
            );

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::new(sandbox.path().join("Cargo.toml")),
                    ..Default::default()
                })
                .await;

            let version =
                |req: &str| ManifestDependency::Version(UnresolvedVersionSpec::parse(req).unwrap());

            assert_eq!(
                output.dependencies,
                BTreeMap::from_iter([
                    ("both".into(), version("^1")),
                    ("exact".into(), version("=1.2.3")),
                    ("libc".into(), version("^0.2")),
                    ("tokio".into(), version("^1.40")),
                ])
            );
            assert_eq!(
                output.dev_dependencies,
                BTreeMap::from_iter([("winapi".into(), version("^0.3.9"))])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package() {
            let sandbox = create_moon_sandbox("files");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::new(sandbox.path().join("package/Cargo.toml")),
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
                        ManifestDependency::Version(
                            UnresolvedVersionSpec::parse("^1.2.3").unwrap()
                        )
                    ),
                    (
                        "e".into(),
                        ManifestDependency::Config(ManifestDependencyConfig {
                            features: vec!["on".into()],
                            version: Some(UnresolvedVersionSpec::parse("^7.8.9").unwrap()),
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
                            version: Some(UnresolvedVersionSpec::parse("^4.5.6").unwrap()),
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
