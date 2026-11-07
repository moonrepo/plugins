use proto_pdk_test_utils::*;

mod proto_tool {
    use super::*;

    generate_resolve_versions_tests!("proto-test", {
        "0.25" => "0.25.3",
        "0.40.0" => "0.40.0",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("proto-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
        // Tags of other crates in the repo, like `version_spec-v0.1.1`
        assert!(output.versions.iter().all(|v| v.as_version().is_some()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_stable_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("proto-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());

        let Some(UnresolvedVersionSpec::Version(latest)) = output.latest else {
            panic!("latest isn't a version");
        };

        assert!(latest.prerelease.is_none() && latest.build.is_none());
    }
}
