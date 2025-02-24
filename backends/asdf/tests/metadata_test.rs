#[cfg(not(windows))]
mod asdf_backend {
    use proto_pdk_test_utils::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("act").await;

        let metadata = plugin
            .register_tool(RegisterToolInput { id: "act".into() })
            .await;

        assert_eq!(metadata.name, "asdf:act");
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_backend() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("act").await;

        let metadata = plugin
            .register_backend(RegisterBackendInput::default())
            .await;

        assert_eq!(metadata.backend_id, "asdf-act");

        if let SourceLocation::Git(git) = metadata.source.unwrap() {
            assert_eq!(git.url, "https://github.com/gr1m0h/asdf-act");
        }
    }
}
