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
        assert!(matches!(metadata.unstable, Switch::Toggle(true)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_inventory_in_jdk_dir() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("java"),
            })
            .await;

        assert_eq!(
            metadata.inventory_options.override_dir_name,
            Some("jdk".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stores_inventory_in_jre_dir_for_jre_id() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("jre").await;

        let metadata = plugin
            .register_tool(RegisterToolInput { id: Id::raw("jre") })
            .await;

        assert_eq!(
            metadata.inventory_options.override_dir_name,
            Some("jre".into())
        );
    }
}
