use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod python_toolchain_tier1 {
    use super::*;

    mod initialize_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_poetry_as_prompt_option() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("python").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput {
                    context: MoonContext {
                        working_dir: plugin.plugin.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    }
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
                    }
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
}
