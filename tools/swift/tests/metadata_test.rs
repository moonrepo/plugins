use proto_pdk_test_utils::*;

mod swift_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("swift-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("swift"),
            })
            .await;

        assert_eq!(metadata.name, "Swift");
        assert_eq!(metadata.minimum_proto_version, Some(Version::new(0, 61, 0)));
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
