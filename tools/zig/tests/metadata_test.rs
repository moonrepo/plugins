use proto_pdk_test_utils::*;

mod zig_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zig-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput { id: Id::raw("zig") })
            .await;

        assert_eq!(metadata.name, "Zig");
        assert_eq!(metadata.type_of, PluginType::Language);
        assert_eq!(metadata.minimum_proto_version, Some(Version::new(0, 61, 0)));
        assert_eq!(metadata.unstable, Switch::Toggle(true));
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn detects_version_files() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zig-test").await;

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
    async fn parses_plain_version_files() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zig-test").await;

        for file in [".zig-version", ".zigversion"] {
            let output = plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "\n0.14.1\n".into(),
                    file: file.into(),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.version,
                Some(UnresolvedVersionSpec::parse("0.14.1").unwrap())
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_zon_minimum_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("zig-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: r#".{
                    .name = .example,
                    .minimum_zig_version = "0.14.1",
                }"#
                .into(),
                file: "build.zig.zon".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version,
            Some(UnresolvedVersionSpec::parse(">=0.14.1").unwrap())
        );
    }
}
