use proto_pdk_test_utils::*;

mod rust_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("rust-test"),
            })
            .await;

        assert_eq!(metadata.name, "Rust");
        assert_eq!(
            metadata.default_version,
            Some(UnresolvedVersionSpec::parse("stable").unwrap())
        );
        assert!(metadata.inventory_options.override_dir.is_some());
        assert!(metadata.inventory_options.version_suffix.is_some());
    }
}
