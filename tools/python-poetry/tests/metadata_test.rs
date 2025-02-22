use proto_pdk_test_utils::*;

mod python_poetry_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("poetry-test").await;

        let metadata = plugin.register_tool(RegisterToolInput::default()).await;

        assert_eq!(metadata.name, "Poetry");
        assert_eq!(metadata.self_upgrade_commands, vec!["self update"]);
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
