use moon_config::{DependencyScope, UnresolvedVersionSpec};
use moon_pdk_api::*;
use moon_pdk_test_utils::{MoonWasmSandbox, ToolchainTestWrapper, create_empty_moon_sandbox};
use serde_json::json;
use std::path::{Path, PathBuf};

/// With workspace config, which `bundlePath` is only read from.
async fn create_ruby_with_config(
    sandbox: &MoonWasmSandbox,
    config: serde_json::Value,
) -> ToolchainTestWrapper {
    sandbox
        .create_toolchain_with_config("ruby", |cfg| {
            cfg.insert("moon_toolchain_config", config.clone());
        })
        .await
}

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
                    starting_dir: VirtualPath::new(sandbox.path()),
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
                    starting_dir: VirtualPath::new(sandbox.path().join("app")),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            // No local gems, so no members are reported.
            assert!(output.members.is_none());
            assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/app"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn finds_lockfile_traversing_upwards() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/Gemfile.lock", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::new(sandbox.path().join("app/lib/nested")),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace/app"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reports_local_gems_as_members() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "Gemfile",
                r#"gem "rails"
gem "billing", path: "libs/billing"
gem "outside", path: "../outside"
gemspec path: "./engines/core/"
path "components" do
  gem "admin_ui"
end
"#,
            );
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .locate_dependencies_root(LocateDependenciesRootInput {
                    starting_dir: VirtualPath::new(sandbox.path().join("libs/billing")),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.root.unwrap(), VirtualPath::new("/workspace"));
            assert_eq!(
                output.members.unwrap(),
                ["libs/billing", "engines/core", "components/admin_ui"]
            );
        }
    }

    mod locate_dependencies_roots {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_each_dir_in_order() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/Gemfile.lock", "");
            sandbox.create_file("api/Gemfile", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .locate_dependencies_roots(LocateDependenciesRootsInput {
                    starting_dirs: vec![
                        VirtualPath::new(sandbox.path().join("app/lib/nested")),
                        VirtualPath::new(sandbox.path().join("other")),
                        VirtualPath::new(sandbox.path().join("api")),
                    ],
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.roots,
                vec![
                    LocateDependenciesRootOutput {
                        members: None,
                        root: Some(VirtualPath::new("/workspace/app")),
                    },
                    LocateDependenciesRootOutput::default(),
                    LocateDependenciesRootOutput {
                        members: None,
                        root: Some(VirtualPath::new("/workspace/api")),
                    },
                ]
            );
        }

        // Directories that share a root reuse its cached manifest
        #[tokio::test(flavor = "multi_thread")]
        async fn shares_roots_between_dirs() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("Gemfile", r#"gem "billing", path: "libs/billing""#);
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .locate_dependencies_roots(LocateDependenciesRootsInput {
                    starting_dirs: vec![
                        VirtualPath::new(sandbox.path().join("libs/billing")),
                        VirtualPath::new(sandbox.path().join("libs/billing/lib")),
                        VirtualPath::new(sandbox.path().join("app")),
                    ],
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            let root = LocateDependenciesRootOutput {
                members: Some(vec!["libs/billing".into()]),
                root: Some(VirtualPath::new("/workspace")),
            };

            assert_eq!(output.roots, vec![root; 3]);
        }
    }

    mod setup_environment {
        use super::*;

        fn path_command(plugin: &ToolchainTestWrapper, root: &Path, path: &str) -> ExecCommand {
            ExecCommand::new(
                ExecCommandInput::new("bundle", ["config", "set", "--local", "path", path])
                    .cwd(plugin.plugin.to_virtual_path(root)),
            )
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn pins_configured_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "version": "3.4" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands[0].command.env.get("PROTO_RUBY_VERSION"),
                // What moon sets, when a task has the toolchain
                Some(&UnresolvedVersionSpec::parse("3.4").unwrap().to_string())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_local_bundle_path() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![path_command(&plugin, sandbox.path(), "vendor/bundle")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_customize_bundle_path() {
            let sandbox = create_empty_moon_sandbox();
            let plugin =
                create_ruby_with_config(&sandbox, json!({ "bundlePath": "vendor/gems" })).await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "bundlePath": "vendor/gems" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![path_command(&plugin, sandbox.path(), "vendor/gems")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_project_bundle_path() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "bundlePath": "vendor/project" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![path_command(&plugin, sandbox.path(), "vendor/bundle")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_persist_frozen() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "frozen": true }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![path_command(&plugin, sandbox.path(), "vendor/bundle")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_if_path_already_configured() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".bundle/config", "---\nBUNDLE_PATH: \"vendor/bundle\"\n");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.commands.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn updates_a_changed_path() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".bundle/config", "---\nBUNDLE_PATH: \"vendor/bundle\"\n");
            let plugin =
                create_ruby_with_config(&sandbox, json!({ "bundlePath": "vendor/gems" })).await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "bundlePath": "vendor/gems" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![path_command(&plugin, sandbox.path(), "vendor/gems")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_path_when_config_has_other_settings() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".bundle/config", "---\nBUNDLE_JOBS: \"4\"\n");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .setup_environment(SetupEnvironmentInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.commands,
                vec![path_command(&plugin, sandbox.path(), "vendor/bundle")]
            );
        }
    }

    mod install_dependencies {
        use super::*;

        fn install_command(
            plugin: &ToolchainTestWrapper,
            root: &Path,
            env: &[(&str, &str)],
        ) -> ExecCommand {
            let mut command = ExecCommandInput::new("bundle", ["install"]);
            command.cwd = Some(plugin.plugin.to_virtual_path(root));

            for (key, value) in env {
                command.env.insert((*key).into(), (*value).into());
            }

            ExecCommand::new(command)
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn freezes_in_ci_with_a_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox
                .create_toolchain_with_config("ruby", |config| {
                    config.host_environment(HostEnvironment {
                        ci: true,
                        ..Default::default()
                    });
                })
                .await;

            let install = async || {
                plugin
                    .install_dependencies(InstallDependenciesInput {
                        root: VirtualPath::new(sandbox.path()),
                        toolchain_config: json!({}),
                        ..Default::default()
                    })
                    .await
                    .install_command
                    .unwrap()
            };

            assert_eq!(
                install().await,
                install_command(&plugin, sandbox.path(), &[])
            );

            sandbox.create_file("Gemfile.lock", "");

            assert_eq!(
                install().await,
                install_command(&plugin, sandbox.path(), &[("BUNDLE_FROZEN", "true")])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn pins_configured_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "version": "3.4.1" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                install_command(&plugin, sandbox.path(), &[("PROTO_RUBY_VERSION", "3.4.1")])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_bundle_install() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.dedupe_command.is_none());
            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("bundle", ["install"])
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
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    production: true,
                    ..Default::default()
                })
                .await;

            let mut expected = ExecCommandInput::new("bundle", ["install"]);
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
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "bundlerInstallArgs": ["--jobs", "4"] }),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.install_command.unwrap(),
                ExecCommand::new(
                    ExecCommandInput::new("bundle", ["install", "--jobs", "4"])
                        .cwd(plugin.plugin.to_virtual_path(sandbox.path()))
                )
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_frozen_env_when_configured() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .install_dependencies(InstallDependenciesInput {
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({ "frozen": true }),
                    ..Default::default()
                })
                .await;

            let mut expected = ExecCommandInput::new("bundle", ["install"]);
            expected.cwd = Some(plugin.plugin.to_virtual_path(sandbox.path()));
            expected.env.insert("BUNDLE_FROZEN".into(), "true".into());

            assert_eq!(output.install_command.unwrap(), ExecCommand::new(expected));
        }
    }

    mod extend_task_command {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_nothing_without_a_root() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("project/bin/rspec", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "rspec".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.paths.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_nothing_without_a_binstubs_dir() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("Gemfile", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "rspec".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(output.paths.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_binstubs_dir_of_the_root() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("Gemfile", "");
            sandbox.create_file("bin/rspec", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: "rspec".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.paths, vec![sandbox.path().join("bin")]);
        }
    }

    mod extend_task_command_bundled_executables {
        use super::*;

        async fn extend(
            command: &str,
            workspace_config: serde_json::Value,
        ) -> ExtendTaskCommandOutput {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("Gemfile", "");
            sandbox.create_file("vendor/bundle/ruby/3.4.0/bin/rspec", "");
            sandbox.create_file("vendor/bundle/ruby/3.4.0/bin/rake", "");
            sandbox.create_file("vendor/bundle/ruby/3.4.0/bin/bundle", "");
            sandbox.create_file("vendor/gems/ruby/3.4.0/bin/rubocop", "");
            sandbox.create_file("bin/rake", "");
            let plugin = create_ruby_with_config(&sandbox, workspace_config).await;

            plugin
                .extend_task_command(ExtendTaskCommandInput {
                    command: command.into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn runs_them_with_bundle_exec() {
            let output = extend("rspec", json!({})).await;

            assert_eq!(output.command.unwrap(), "bundle");
            assert_eq!(
                output.args.unwrap(),
                Extend::Prepend(vec!["exec".into(), "rspec".into()])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_configured_bundle_path() {
            let output = extend("rubocop", json!({ "bundlePath": "vendor/gems" })).await;

            assert_eq!(output.command.unwrap(), "bundle");

            let output = extend("rubocop", json!({})).await;

            assert!(output.command.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn prefers_binstubs() {
            let output = extend("rake", json!({})).await;

            assert!(output.command.is_none());
            assert!(output.args.is_none());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_bundler_and_paths() {
            assert!(extend("bundle", json!({})).await.command.is_none());
            assert!(extend("./rspec", json!({})).await.command.is_none());
        }
    }

    mod extend_task_script {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn adds_binstubs_dir_of_the_root() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("project/Gemfile", "");
            sandbox.create_file("project/bin/rails", "");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .extend_task_script(ExtendTaskScriptInput {
                    script: "bin/rails test".into(),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.paths, vec![sandbox.path().join("project/bin")]);
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
                    path: VirtualPath::new(sandbox.path().join("Gemfile.lock")),
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
                    path: VirtualPath::new(sandbox.path().join("Gemfile.lock")),
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
            assert_eq!(fork[0].hash.as_deref(), Some("abc123def456"));

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

    mod parse_lock_versions {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn normalizes_rubygems_versions() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "Gemfile.lock",
                r#"GEM
  remote: https://rubygems.org/
  specs:
    actionpack (7.1.3.4)
    minitest (5.25)
    rack (3)
    tzinfo-data (1.2024.01)
    shakapacker (8.0.0.pre)
    rails (8.1.0.beta1.2)

PLATFORMS
  ruby

DEPENDENCIES
  rails
"#,
            );
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .parse_lock(ParseLockInput {
                    path: VirtualPath::new(sandbox.path().join("Gemfile.lock")),
                    ..Default::default()
                })
                .await;

            for (name, version) in [
                ("actionpack", "7.1.3+4"),
                ("minitest", "5.25.0"),
                ("rack", "3.0.0"),
                ("tzinfo-data", "1.2024.1"),
                ("shakapacker", "8.0.0+pre"),
                ("rails", "8.1.0+beta1.2"),
            ] {
                assert_eq!(
                    output.dependencies.get(name).unwrap()[0].version,
                    Some(VersionSpec::parse(version).unwrap()),
                    "{name}"
                );
            }
        }
    }

    mod parse_manifest {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn reports_literal_gems() {
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
                    path: VirtualPath::new(sandbox.path().join("Gemfile")),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.dependencies.get("rails"),
                Some(&ManifestDependency::new(
                    UnresolvedVersionSpec::parse("*").unwrap()
                ))
            );
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
        async fn reads_gems_rb() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("app/gems.rb", r#"gem "billing", path: "../libs/billing""#);
            let plugin = sandbox.create_toolchain("ruby").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("app"), "app".into());
            input
                .project_sources
                .insert(Id::raw("billing"), "libs/billing".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects.get("app").unwrap().dependencies[0].id,
                Id::raw("billing")
            );
            assert_eq!(
                output.input_files,
                vec![
                    plugin
                        .plugin
                        .to_virtual_path(sandbox.path().join("app/gems.rb"))
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn infers_gemspec_dependency_edges() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("Gemfile", r#"gem "billing", path: "libs/billing""#);
            sandbox.create_file(
                "libs/billing/billing.gemspec",
                r#"Gem::Specification.new do |spec|
  spec.name = "billing"
  spec.add_dependency "core"
  spec.add_development_dependency "test_support"
  spec.add_dependency "rails"
end
"#,
            );
            sandbox.create_file("libs/core/core.gemspec", "");
            sandbox.create_file(
                "libs/support/test_support.gemspec",
                r#"Gem::Specification.new do |s|
  s.name = "test_support"
end"#,
            );
            let plugin = sandbox.create_toolchain("ruby").await;

            let mut input = ExtendProjectGraphInput::default();
            input
                .project_sources
                .insert(Id::raw("billing"), "libs/billing".into());
            input
                .project_sources
                .insert(Id::raw("core"), "libs/core".into());
            input
                .project_sources
                .insert(Id::raw("support"), "libs/support".into());

            let output = plugin.extend_project_graph(input).await;
            let billing = output.extended_projects.get("billing").unwrap();

            assert_eq!(billing.alias.as_deref(), Some("billing"));
            assert_eq!(
                billing.dependencies,
                vec![
                    ProjectDependency {
                        id: Id::raw("core"),
                        scope: DependencyScope::Production,
                        via: Some("gemspec dependency core".into()),
                    },
                    ProjectDependency {
                        id: Id::raw("support"),
                        scope: DependencyScope::Development,
                        via: Some("gemspec dependency test_support".into()),
                    },
                ]
            );
            assert_eq!(
                output
                    .extended_projects
                    .get("support")
                    .unwrap()
                    .alias
                    .as_deref(),
                Some("test_support")
            );
            assert!(
                output.input_files.contains(
                    &plugin
                        .plugin
                        .to_virtual_path(sandbox.path().join("libs/billing/billing.gemspec"))
                )
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn resolves_path_gems_by_gem_name() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "app/Gemfile",
                r#"path "../components" do
  gem "admin_ui"
end"#,
            );
            sandbox.create_file(
                "components/admin/admin_ui.gemspec",
                r#"spec.name = "admin_ui""#,
            );
            let plugin = sandbox.create_toolchain("ruby").await;

            let mut input = ExtendProjectGraphInput::default();
            input.project_sources.insert(Id::raw("app"), "app".into());
            input
                .project_sources
                .insert(Id::raw("admin"), "components/admin".into());

            let output = plugin.extend_project_graph(input).await;

            assert_eq!(
                output.extended_projects.get("app").unwrap().dependencies[0].id,
                Id::raw("admin")
            );
        }

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
}
