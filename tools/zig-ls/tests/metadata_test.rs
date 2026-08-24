use proto_pdk_test_utils::*;

mod zig_ls_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zls-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput { id: Id::raw("zls") })
            .await;

        assert_eq!(metadata.name, "ZLS");
        assert_eq!(metadata.type_of, PluginType::CommandLine);
        assert_eq!(metadata.minimum_proto_version, Some(Version::new(0, 61, 0)));
        assert_eq!(metadata.unstable, Switch::Toggle(true));
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
