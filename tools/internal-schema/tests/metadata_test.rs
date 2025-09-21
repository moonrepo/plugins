use proto_pdk_test_utils::*;
use starbase_sandbox::locate_fixture;

mod schema_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin("schema-test", locate_fixture("schemas").join("base.toml"))
            .await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("moon-test"),
            })
            .await;

        assert_eq!(metadata.name, "moon-test");
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
