use proto_pdk_test_utils::*;

mod rust_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        let metadata = plugin
            .register_tool(ToolMetadataInput {
                id: "rust-test".into(),
            })
            .await;

        assert_eq!(metadata.name, "Rust");
        assert_eq!(
            metadata.default_version,
            Some(UnresolvedVersionSpec::parse("stable").unwrap())
        );
        assert!(metadata.inventory.override_dir.is_some());
        assert!(metadata.inventory.version_suffix.is_some());
    }
}
