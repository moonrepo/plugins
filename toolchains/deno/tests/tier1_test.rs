use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod bun_toolchain_tier1 {
    use super::*;

    mod define_docker_metadata {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_image_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("bun").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "oven/bun:latest");

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({
                        "version": "1.2"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "oven/bun:1.2");
        }
    }
}
