use proto_pdk_test_utils::*;

mod swift_tool {
    use super::*;

    generate_resolve_versions_tests!("swift-test", {
        "6.2" => "6.2.4",
        "6.1" => "6.1.3",
        "6.1.2" => "6.1.2",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("swift-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("swift-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_swift_version_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("swift-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "\n6.1.2\n".into(),
                file: ".swift-version".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("6.1.2").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_package_swift_tools_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("swift-test").await;

        for (content, expected) in [
            (
                "// swift-tools-version:5.9\nimport PackageDescription",
                "^5.9",
            ),
            ("// swift-tools-version: 6.0\n", "^6.0"),
            ("//swift-tools-version:5.10", "^5.10"),
            ("//   Swift-Tools-Version:  6.1", "^6.1"),
            ("// swift-tools-version: 6.3;(experimentalCGen)", "^6.3"),
        ] {
            let output = plugin
                .parse_version_file(ParseVersionFileInput {
                    content: content.into(),
                    file: "Package.swift".into(),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.version.unwrap(),
                UnresolvedVersionSpec::parse(expected).unwrap(),
                "{content}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_no_version_from_package_swift() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("swift-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "import PackageDescription\n\nlet package = Package(name: \"a\")".into(),
                file: "Package.swift".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(output.version, None);
    }
}
