use moon_config::{DockerPruneConfig, LanguageType};
use moon_pdk_api::*;
use moon_pdk_test_utils::{MoonWasmSandbox, create_empty_moon_sandbox};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

mod ruby_toolchain_tier1 {
    use super::*;

    mod register_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn reports_ruby_metadata() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .register_toolchain(RegisterToolchainInput {
                    id: Id::raw("ruby"),
                })
                .await;

            assert_eq!(output.name, "Ruby");
            assert_eq!(output.language, Some(LanguageType::Ruby));
            assert_eq!(output.vendor_dir_name.unwrap(), "vendor/bundle");
            assert!(output.manifest_file_names.contains(&"Gemfile".to_string()));
            assert!(output.lock_file_names.contains(&"Gemfile.lock".to_string()));
            assert!(output.exe_names.contains(&"bundle".to_string()));
            assert!(output.exe_names.contains(&"rspec".to_string()));
            assert!(output.config_file_globs.contains(&"Rakefile".to_string()));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reports_configured_vendor_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox
                .create_toolchain_with_config("ruby", |config| {
                    config.insert(
                        "moon_toolchain_config",
                        json!({ "bundlePath": "vendor/gems" }),
                    );
                })
                .await;

            assert_eq!(plugin.metadata.vendor_dir_name.unwrap(), "vendor/gems");
        }
    }

    mod initialize_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn prefills_version_from_ruby_version_file() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".ruby-version", "ruby-3.3.5\n");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                })
                .await;

            assert_eq!(output.default_settings.get("version").unwrap(), "3.3.5");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn ignores_other_engines() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(".ruby-version", "jruby-9.4.5.0\n");
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                })
                .await;

            assert!(!output.default_settings.contains_key("version"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_version_without_file() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    },
                })
                .await;

            assert!(!output.default_settings.contains_key("version"));
        }
    }

    mod define_docker_metadata {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_image_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "ruby:latest");

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({ "version": "3.3" }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "ruby:3.3");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn scaffolds_gem_version_files() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("ruby").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput::default())
                .await;

            assert_eq!(output.scaffold_globs, ["lib/**/version.rb"]);
        }
    }

    mod prune_docker {
        use super::*;

        // Requires a working Bundler (proto's shim runs the system `ruby` instead
        // when no Ruby is pinned), and returns the gem directory of its Ruby ABI
        fn get_ruby_abi(dir: &Path) -> Option<String> {
            let run = |args: &[&str]| {
                Command::new("bundle")
                    .args(args)
                    .current_dir(dir)
                    .output()
                    .ok()
                    .filter(|out| out.status.success())
                    .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            };

            run(&["-v"])?.starts_with("Bundler").then_some(())?;

            run(&[
                "exec",
                "ruby",
                "-e",
                "print RbConfig::CONFIG['ruby_version']",
            ])
        }

        async fn prune(
            delete_vendor_directories: bool,
        ) -> Option<(MoonWasmSandbox, PruneDockerOutput)> {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("Gemfile", "source \"https://rubygems.org\"\n");
            sandbox.create_file(".bundle/config", "---\nBUNDLE_PATH: \"vendor/bundle\"\n");

            let abi = get_ruby_abi(sandbox.path())?;
            sandbox.create_file(
                format!("vendor/bundle/ruby/{abi}/specifications/stale-1.0.gemspec"),
                "",
            );
            sandbox.create_file(
                format!("vendor/bundle/ruby/{abi}/gems/stale-1.0/stale.rb"),
                "",
            );

            let plugin = sandbox.create_toolchain("ruby").await;
            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories,
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            Some((sandbox, output))
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn persists_groups_and_cleans_their_gems() {
            let Some((sandbox, output)) = prune(true).await else {
                return;
            };

            assert!(
                fs::read_to_string(sandbox.path().join(".bundle/config"))
                    .unwrap()
                    .contains("BUNDLE_WITHOUT: \"development:test\"")
            );
            assert!(
                fs::read_dir(sandbox.path().join("vendor/bundle/ruby"))
                    .unwrap()
                    .all(|abi| !abi.unwrap().path().join("gems/stale-1.0").exists())
            );
            assert_eq!(output.changed_files.len(), 2);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn only_persists_groups_when_not_deleting() {
            let Some((sandbox, output)) = prune(false).await else {
                return;
            };

            assert!(
                fs::read_to_string(sandbox.path().join(".bundle/config"))
                    .unwrap()
                    .contains("BUNDLE_WITHOUT: \"development:test\"")
            );
            assert!(
                fs::read_dir(sandbox.path().join("vendor/bundle/ruby"))
                    .unwrap()
                    .any(|abi| abi.unwrap().path().join("gems/stale-1.0").exists())
            );
            assert_eq!(output.changed_files.len(), 1);
        }
    }
}
