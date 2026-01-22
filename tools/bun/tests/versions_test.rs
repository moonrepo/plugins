use proto_pdk_test_utils::*;

mod bun_tool {
    use super::*;

    generate_resolve_versions_tests!("bun-test", {
        "0.4" => "0.4.0",
        "0.5.1" => "0.5.1",
        "1.1.0" => "1.1.0",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_engines() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: r#"{ "engines": { "bun": ">=1" } }"#.into(),
                    file: "package.json".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_dev_engines() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content:
                        r#"{ "devEngines": { "runtime": { "name": "bun", "version": ">=1" } } }"#
                            .into(),
                    file: "package.json".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_volta() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: r#"{ "volta": { "bun": "1.20.2" } }"#.into(),
                    file: "package.json".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.20.2").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_bumrc() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "~2".into(),
                    file: ".bumrc".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("~2").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_bumrc_with_comment() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "# comment\n^2.1".into(),
                    file: ".bumrc".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("^2.1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_bun_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "~2".into(),
                    file: ".bun-version".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("~2").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_bun_version_with_comment() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "# comment\n^2.1".into(),
                    file: ".bun-version".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("^2.1").unwrap()),
            }
        );
    }
}
