use proto_pdk_test_utils::*;
use starbase_sandbox::locate_fixture;

fn version_file(
    sandbox: &ProtoWasmSandbox,
    file: &str,
    content: impl AsRef<str>,
) -> ParseVersionFileInput {
    sandbox.create_file(file, content);

    ParseVersionFileInput {
        file: file.into(),
        path: VirtualPath::new(sandbox.path().join(file)),
        ..Default::default()
    }
}

mod schema_tool {
    use super::*;

    generate_resolve_versions_tests!(
        "schema-test",
        {
            "1.0.3" => "1.0.3",
            "1.4" => "1.4.0",
            "1.5" => "1.5.1",
        },
        Some(locate_fixture("schemas").join("base.toml"))
    );

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git_tags() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin("schema-test", locate_fixture("schemas").join("base.toml"))
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_index() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin("schema-test", locate_fixture("schemas").join("index.toml"))
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(
            output
                .versions
                .contains(&VersionSpec::parse("20.0.0").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_index_path() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin(
                "schema-test",
                locate_fixture("schemas").join("v2-index-path.toml"),
            )
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(
            output
                .versions
                .contains(&VersionSpec::parse("0.14.0").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_version_file_without_comments() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin("schema-test", locate_fixture("schemas").join("base.toml"))
            .await;

        let output = plugin
            .parse_version_file(version_file(
                &sandbox,
                ".moon-version",
                "# pinned for CI\n1.4 # stable\n",
            ))
            .await;

        assert_eq!(
            output.version,
            Some(UnresolvedVersionSpec::parse("1.4").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin("schema-test", locate_fixture("schemas").join("base.toml"))
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn version_pattern_supports_common_classes() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin(
                "schema-test",
                locate_fixture("schemas").join("version-pattern.toml"),
            )
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }
}
