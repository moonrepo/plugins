use proto_pdk_test_utils::*;

mod java_tool {
    use super::*;

    generate_resolve_versions_tests!("java-test", {
        "17" => "17.0.19",
        "21" => "21.0.11",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_java_version_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "21.0.11\n".into(),
                file: ".java-version".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("21.0.11").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_sdkmanrc_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "java=21.0.11-tem\n".into(),
                file: ".sdkmanrc".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("21.0.11").unwrap()
        );
    }
}
