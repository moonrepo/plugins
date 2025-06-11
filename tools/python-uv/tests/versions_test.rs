use proto_pdk_test_utils::*;

mod python_uv_tool {
    use super::*;

    generate_resolve_versions_tests!("uv-test", {
        "0.3" => "0.3.5",
        "0.5.21" => "0.5.21",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("uv-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("uv-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_uv_toml() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("uv-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: r#"required-version = "==2.1""#.into(),
                    file: "uv.toml".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("=2.1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_pyproject_toml() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("uv-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "[tool.uv]\nrequired-version = \"~=2.1\"".into(),
                    file: "pyproject.toml".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("~2.1").unwrap()),
            }
        );
    }
}
