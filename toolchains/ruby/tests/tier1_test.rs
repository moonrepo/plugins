use moon_config::LanguageType;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

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

            assert!(output.default_settings.get("version").is_none());
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
    }
}
