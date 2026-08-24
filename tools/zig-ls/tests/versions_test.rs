use proto_pdk_test_utils::*;

mod zig_ls_tool {
    use super::*;

    generate_resolve_versions_tests!("zls-test", {
        "0.16" => "0.16.0",
        "0.15" => "0.15.1",
        "0.14.0" => "0.14.0",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_stable_versions() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zls-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
        assert!(output.latest.is_some());
        assert!(output.canary.is_none());
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }
}
