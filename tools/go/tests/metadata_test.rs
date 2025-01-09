use proto_pdk_test_utils::*;

mod go_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;

        let metadata = plugin.register_tool(ToolMetadataInput::default()).await;

        assert_eq!(metadata.name, "Go");
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
