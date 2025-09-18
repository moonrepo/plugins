use proto_pdk_test_utils::*;

mod cargo_backend_metadata {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("cargo-nextest").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: "cargo-nextest".into(),
            })
            .await;

        assert_eq!(metadata.name, "cargo:cargo-nextest");
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(metadata.requires, ["rust"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_backend() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("cargo-nextest").await;

        let metadata = plugin
            .register_backend(RegisterBackendInput::default())
            .await;

        assert_eq!(metadata.backend_id, "cargo-nextest");
    }
}
