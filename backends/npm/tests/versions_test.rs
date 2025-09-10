use proto_pdk_test_utils::*;

mod npm_backend {
    use super::*;

    generate_resolve_versions_tests!("npm:typescript", {
        "5.7" => "5.7.3",
        "5.9.2" => "5.9.2",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_npm_registry() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("npm:typescript").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }
}
