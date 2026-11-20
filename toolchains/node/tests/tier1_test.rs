use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod node_toolchain_tier1 {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_version_files_as_config_files() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_toolchain("node").await;

        let output = plugin
            .register_toolchain(RegisterToolchainInput {
                id: Id::raw("node"),
            })
            .await;

        assert_eq!(output.config_file_globs, [".nvmrc", ".node-version"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_prompt_for_unused_settings() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_toolchain("node").await;

        let output = plugin
            .initialize_toolchain(InitializeToolchainInput::default())
            .await;

        assert!(output.prompts.is_empty());
    }

    mod define_docker_metadata {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_image_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("node").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "node:lts");

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({
                        "version": "20"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "node:20");

            for (version, image) in [
                (">=20", "node:latest"),
                ("lts", "node:lts"),
                ("stable", "node:lts"),
                ("lts-jod", "node:jod"),
                ("lts/jod", "node:jod"),
                ("jod", "node:jod"),
                ("node", "node:latest"),
                ("canary", "node:latest"),
                ("~22.22", "node:22.22"),
                ("22.22.3", "node:22.22.3"),
                ("27.0.0-alpha.0.1.1", "node:latest"),
                ("^27.0.0-alpha", "node:latest"),
            ] {
                let output = plugin
                    .define_docker_metadata(DefineDockerMetadataInput {
                        toolchain_config: json!({ "version": version }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(output.default_image.unwrap(), image, "{version}");
            }
        }
    }
}
