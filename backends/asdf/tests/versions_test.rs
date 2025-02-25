#[cfg(not(windows))]
mod asdf_backend {
    use proto_pdk_test_utils::*;

    generate_resolve_versions_tests!("asdf:zig", {
        "asdf:0.12" => "0.12.1",
        "asdf:0.13.0" => "0.13.0",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_backend("zig", Backend::Asdf)
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }
}
