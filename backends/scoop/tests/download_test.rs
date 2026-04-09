#[cfg(windows)]
mod scoop_backend {
    use proto_pdk_test_utils::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_prebuilt() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("scoop:jq").await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: ToolContext {
                    version: VersionSpec::parse("1.7.1").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert!(!output.download_url.is_empty());
        assert!(output.download_url.contains("1.7.1"));
    }
}
