use proto_pdk_test_utils::*;

mod java_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("java"),
            })
            .await;

        assert_eq!(metadata.name, "Java");
        assert_eq!(metadata.minimum_proto_version, Some(Version::new(0, 59, 0)));
    }
}
