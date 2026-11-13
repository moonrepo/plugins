use proto_pdk_test_utils::*;

mod pkl_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("pkl-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput { id: Id::raw("pkl") })
            .await;

        assert_eq!(metadata.name, "Pkl");
        assert!(metadata.self_upgrade_commands.is_empty());
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
