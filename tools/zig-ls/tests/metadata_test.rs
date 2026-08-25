use proto_pdk_test_utils::*;

mod zig_ls_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zls-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput { id: Id::raw("zls") })
            .await;

        assert_eq!(metadata.name, "ZLS");
        assert_eq!(metadata.type_of, PluginType::CommandLine);
        assert_eq!(metadata.minimum_proto_version, Some(Version::new(0, 61, 0)));
        assert_eq!(metadata.unstable, Switch::Toggle(true));
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn detects_zig_version_files() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zls-test").await;

        assert_eq!(
            plugin
                .detect_version_files(DetectVersionInput::default())
                .await,
            DetectVersionOutput {
                files: vec![
                    ".zig-version".into(),
                    ".zigversion".into(),
                    "build.zig.zon".into(),
                ],
                ignore: vec![".zig-cache".into(), "zig-out".into()],
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_plain_zig_version_files_as_zls_minor_ranges() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zls-test").await;

        for file in [".zig-version", ".zigversion"] {
            let output = plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "\n0.15.2\n".into(),
                    file: file.into(),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.version,
                Some(UnresolvedVersionSpec::parse("~0.15").unwrap())
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_zon_minimum_version_as_zls_minimum_range() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zls-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: r#".{
                    .name = .example,
                    .minimum_zig_version = "0.15.2",
                }"#
                .into(),
                file: "build.zig.zon".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version,
            Some(UnresolvedVersionSpec::parse(">=0.15").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn maps_development_zig_versions_to_canary() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zls-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "0.16.0-dev.123+abc".into(),
                file: ".zig-version".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version,
            Some(UnresolvedVersionSpec::parse("canary").unwrap())
        );
    }
}
