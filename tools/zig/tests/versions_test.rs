use proto_pdk_test_utils::*;

mod zig_tool {
    use super::*;

    generate_resolve_versions_tests!("zig-test", {
        "0.14" => "0.14.1",
        "0.13" => "0.13.0",
        "0.12.1" => "0.12.1",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_stable_and_master_versions() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zig-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
        assert!(output.latest.is_some());
        assert!(output.canary.is_some());
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
        assert_eq!(output.aliases.get("master"), output.canary.as_ref());
    }
}
