mod scoop_backend {
    use proto_pdk_test_utils::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("scoop:jq").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert!(output.aliases.contains_key("stable"));
    }
}
