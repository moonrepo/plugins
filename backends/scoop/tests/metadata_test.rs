mod scoop_backend {
    use proto_pdk_test_utils::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("jq").await;

        let metadata = plugin
            .register_tool(RegisterToolInput { id: Id::raw("jq") })
            .await;

        assert_eq!(metadata.name, "scoop:jq");
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata_as_scoop() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("scoop").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("scoop"),
            })
            .await;

        assert_eq!(metadata.name, "scoop");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_backend() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("jq").await;

        let metadata = plugin
            .register_backend(RegisterBackendInput {
                id: Id::raw("jq"),
                ..Default::default()
            })
            .await;

        assert_eq!(metadata.backend_id, "jq");
        assert!(metadata.source.is_none());
    }
}
