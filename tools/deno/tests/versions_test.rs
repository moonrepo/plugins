use proto_pdk_test_utils::*;

mod deno_tool {
    use super::*;

    generate_resolve_versions_tests!("deno-test", {
        "1.19" => "1.19.3",
        "1.11" => "1.11.5",
        "1.9.2" => "1.9.2",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("deno-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("deno-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }
}
