use npm_backend::NpmBackendConfig;
use proto_pdk_test_utils::*;

mod npm_backend_metadata {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("typescript").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: "typescript".into(),
            })
            .await;

        assert_eq!(metadata.name, "npm:typescript");
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_backend() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("typescript").await;

        let metadata = plugin
            .register_backend(RegisterBackendInput::default())
            .await;

        assert_eq!(metadata.backend_id, "typescript");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn requires_node_npm() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("typescript").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: "typescript".into(),
            })
            .await;

        assert_eq!(metadata.requires, ["node", "npm"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn requires_bun() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("typescript", |cfg| {
                cfg.backend_config(NpmBackendConfig { bun: true });
            })
            .await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: "typescript".into(),
            })
            .await;

        assert_eq!(metadata.requires, ["bun"]);
    }
}
