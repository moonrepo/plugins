use proto_pdk_test_utils::*;

mod npm_backend {
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
}
