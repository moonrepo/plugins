use proto_pdk_test_utils::*;

mod rust_tool {
    use super::*;

    generate_resolve_versions_tests!("rust-test", {
        "stable" => "stable",
        "nightly" => "nightly",
        "nightly-2023-07-03" => "nightly-2023-07-03",
        "1.60" => "1.60.0",
        "1.71.1" => "1.71.1",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git_tags() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_rust_toolchain() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "1.60.0".into(),
                    file: "rust-toolchain".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.60.0").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ignores_empty_rust_toolchain() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "".into(),
                    file: "rust-toolchain".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput { version: None }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_rust_toolchain_toml() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "[toolchain]\nchannel = \"1.70.0\"".into(),
                    file: "rust-toolchain.toml".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.70.0").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ignores_empty_rust_toolchain_toml() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "[toolchain]".into(),
                    file: "rust-toolchain.toml".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput { version: None }
        );
    }
}
