use moon_config::DependencyScope;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;
use std::path::PathBuf;

mod ruby_toolchain_tier2 {
    use super::*;

    mod locate_dependencies_root {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_if_nothing_found() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.members.is_none());
            assert!(output.root.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_gemfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/Gemfile", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("app")),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            // Bundler has no workspaces, so no members are reported.
            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/app"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_lockfile_traversing_upwards() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/Gemfile.lock", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::Real(sandbox.path().join("app/lib/nested")),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.root.unwrap(), PathBuf::from("/workspace/app"));
        }
    }

    mod setup_environment {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_local_bundle_path() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new(
                        "bundler",
                        ["config", "set", "--local", "path", "vendor/bundle"]
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_customize_bundle_path() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({ "bundlePath": "vendor/gems" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![ExecCommand::new(
                    ExecCommandInput::new(
                        "bundler",
                        ["config", "set", "--local", "path", "vendor/gems"]
                    )
                    .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_frozen_command_when_configured() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({ "frozen": true }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "bundler",
                            ["config", "set", "--local", "path", "vendor/bundle"]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    ),
                    ExecCommand::new(
                        ExecCommandInput::new(
                            "bundler",
                            ["config", "set", "--local", "frozen", "true"]
                        )
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                    )
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_if_already_configured() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".bundle/config", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }
    }

    mod install_dependencies {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_bundle_install() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.dedupe_command.is_none());
            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("bundler", ["install"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn excludes_groups_for_production() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({}),
                    production: true,
                    ..Default::default()
                })
                .await;

            let mut expected = ExecCommandInput::new("bundler", ["install"]);
            expected.cwd = Some(plugin.plugin.to_virtual_path(sandbox.path()));
            expected
                .env
                .insert("BUNDLE_WITHOUT".into(), "development:test".into());

            assert_eq!(output.install_command.unwrap(), ExecCommand::new(expected));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn appends_custom_args() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({ "bundlerInstallArgs": ["--jobs", "4"] }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("bundler", ["install", "--jobs", "4"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }
    }

    mod extend_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_nothing_without_a_root() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .extend_command(ExtendCommandInput {
                    command: "rake".into(),
                    toolchain_config: json!({}),
                    current_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert!(output.paths.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_binstubs_dir_when_root_found() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("Gemfile", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .extend_command(ExtendCommandInput {
                    command: "rake".into(),
                    toolchain_config: json!({}),
                    current_dir: plugin
                        .plugin
                        .to_virtual_path(sandbox.path().join("sub/dir")),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.paths, vec![sandbox.path().join("bin")]);
        }
    }
}

mod ruby_toolchain_tier2_ecosystem {
    use super::*;

    mod parse_lock {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn maps_specs_to_dependencies() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "Gemfile.lock",
                r#"GEM
  remote: https://rubygems.org/
  specs:
    rake (13.0.6)

PLATFORMS
  ruby

DEPENDENCIES
  rake

BUNDLED WITH
   2.5.6
"#,
            );
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("Gemfile.lock")),
                    ..Default::default()
                })
                .await;

            let rake = output.dependencies.get("rake").unwrap();
            assert_eq!(rake.len(), 1);
            assert_eq!(rake[0].version, Some(VersionSpec::parse("13.0.6").unwrap()));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn maps_sources_platforms_and_checksums() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "Gemfile.lock",
                r#"PATH
  remote: ../libs/billing
  specs:
    billing (0.1.0)

GIT
  remote: https://github.com/me/some_fork.git
  revision: abc123def456
  specs:
    some_fork (1.2.0)

GEM
  remote: https://rubygems.org/
  specs:
    nokogiri (1.16.0-x86_64-linux)
    rake (13.0.6)

PLATFORMS
  x86_64-linux

DEPENDENCIES
  billing!
  nokogiri
  rake
  some_fork!

CHECKSUMS
  rake (13.0.6) sha256=deadbeef

BUNDLED WITH
   2.5.6
"#,
            );
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::Real(sandbox.path().join("Gemfile.lock")),
                    ..Default::default()
                })
                .await;

            let billing = output.dependencies.get("billing").unwrap();
            assert_eq!(
                billing[0].version,
                Some(VersionSpec::parse("0.1.0").unwrap())
            );
            assert_eq!(billing[0].meta.as_deref(), Some("path:../libs/billing"));

            let fork = output.dependencies.get("some_fork").unwrap();
            assert_eq!(fork[0].meta.as_deref(), Some("abc123def456"));

            let nokogiri = output.dependencies.get("nokogiri").unwrap();
            assert_eq!(
                nokogiri[0].version,
                Some(VersionSpec::parse("1.16.0").unwrap())
            );
            assert_eq!(nokogiri[0].meta.as_deref(), Some("x86_64-linux"));

            let rake = output.dependencies.get("rake").unwrap();
            assert_eq!(rake[0].hash.as_deref(), Some("deadbeef"));
        }
    }

    mod parse_manifest {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn reports_only_local_path_gems() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "Gemfile",
                r#"source "https://rubygems.org"
gem "rails", "7.1.3"
gem "billing", path: "../libs/billing"
group :ci do
  gem "ci_support", path: "../libs/ci_support"
end
group :test do
  gem "test_support", path: "../libs/test_support"
end
"#,
            );
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .parse_manifest(ParseManifestInput {
                    path: VirtualPath::Real(sandbox.path().join("Gemfile")),
                    ..Default::default()
                })
                .await;

            assert!(!output.dependencies.contains_key("rails"));
            assert_eq!(
                output.dependencies.get("billing"),
                Some(&ManifestDependency::path(PathBuf::from("../libs/billing")))
            );
            // Unknown groups are not guessed as development by the manifest
            // parser; only the conventional dev/test groups are collapsed here.
            assert_eq!(
                output.dependencies.get("ci_support"),
                Some(&ManifestDependency::path(PathBuf::from(
                    "../libs/ci_support"
                )))
            );
            assert_eq!(
                output.dev_dependencies.get("test_support"),
                Some(&ManifestDependency::path(PathBuf::from(
                    "../libs/test_support"
                )))
            );
        }
    }

    mod extend_project_graph {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn infers_path_gem_edges() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/Gemfile", r#"gem "billing", path: "../libs/billing""#);
            let plugin = sandbox.create_toolchain("ruby").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("app"), "app".into());
            input
                .project_sources
                .insert(Id::raw("billing"), "libs/billing".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects.get("app").unwrap().dependencies,
                vec![ProjectDependency {
                    id: Id::raw("billing"),
                    scope: DependencyScope::Production,
                    via: Some("path gem billing".into()),
                }]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_parent_directory_path_gem_edges() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("spree/admin/Gemfile", r#"gem 'spree', path: '../'"#);
            let plugin = sandbox.create_toolchain("ruby").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert(Id::raw("spree"), "spree".into());
            input
                .project_sources
                .insert(Id::raw("spree-admin"), "spree/admin".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output
                    .extended_projects
                    .get("spree-admin")
                    .unwrap()
                    .dependencies,
                vec![ProjectDependency {
                    id: Id::raw("spree"),
                    scope: DependencyScope::Production,
                    via: Some("path gem spree".into()),
                }]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn applies_configured_non_production_groups() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "app/Gemfile",
                r#"group :ci do
  gem "fixtures", path: "../libs/fixtures"
end"#,
            );
            let plugin = sandbox.create_toolchain("ruby").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("app"), "app".into());
            input
                .project_sources
                .insert(Id::raw("fixtures"), "libs/fixtures".into());
            input.toolchain_config = json!({ "productionWithoutGroups": ["ci"] });

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects.get("app").unwrap().dependencies,
                vec![ProjectDependency {
                    id: Id::raw("fixtures"),
                    scope: DependencyScope::Development,
                    via: Some("path gem fixtures".into()),
                }]
            );
        }
    }

    mod hash_task_contents {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_resolved_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .hash_task_contents(HashTaskContentsInput {
                    toolchain_config: json!({ "version": "3.3.5" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.contents.len(), 1);
            assert_eq!(output.contents[0].get("version").unwrap(), "3.3.5");
        }
    }
}
