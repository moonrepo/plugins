use proto_pdk_test_utils::*;

mod node_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        let metadata = plugin.register_tool(ToolMetadataInput::default()).await;

        assert_eq!(metadata.name, "Node.js");
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
