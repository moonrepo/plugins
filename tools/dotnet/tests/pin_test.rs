use proto_pdk_test_utils::*;
use std::fs;

mod dotnet_tool {
    use super::*;

    fn read_global(sandbox: &ProtoWasmSandbox) -> String {
        fs::read_to_string(sandbox.path().join("global.json")).unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pins_the_sdk_version() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("global.json", r#"{"sdk": {"rollForward": "disable"}}"#);

        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin
            .pin_version(PinVersionInput {
                dir: plugin.tool.to_virtual_path(sandbox.path()),
                version: UnresolvedVersionSpec::parse("8.0.404").unwrap(),
                ..Default::default()
            })
            .await;

        assert!(output.pinned);
        assert!(read_global(&sandbox).contains(r#""version": "8.0.404""#));
        // An existing setting is not ours to discard.
        assert!(read_global(&sandbox).contains("rollForward"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_the_sdk_entry_when_absent() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("global.json", r#"{"msbuild-sdks": {"A": "1.0.0"}}"#);

        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin
            .pin_version(PinVersionInput {
                dir: plugin.tool.to_virtual_path(sandbox.path()),
                version: UnresolvedVersionSpec::parse("10.0.400").unwrap(),
                ..Default::default()
            })
            .await;

        assert!(output.pinned);

        let content = read_global(&sandbox);

        assert!(content.contains(r#""version": "10.0.400""#));
        assert!(content.contains("msbuild-sdks"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reports_a_missing_global_json() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin
            .pin_version(PinVersionInput {
                dir: plugin.tool.to_virtual_path(sandbox.path()),
                version: UnresolvedVersionSpec::parse("8.0.404").unwrap(),
                ..Default::default()
            })
            .await;

        assert!(!output.pinned);
        assert!(output.error.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unpins_and_returns_the_previous_version() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("global.json", r#"{"sdk": {"version": "8.0.404"}}"#);

        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin
            .unpin_version(UnpinVersionInput {
                dir: plugin.tool.to_virtual_path(sandbox.path()),
                ..Default::default()
            })
            .await;

        assert!(output.unpinned);
        assert_eq!(
            output.version.map(|version| version.to_string()).as_deref(),
            Some("8.0.404")
        );
        // Nothing else was in `sdk`, so it goes too rather than being left empty.
        assert!(!read_global(&sandbox).contains("sdk"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unpinning_keeps_other_sdk_settings() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            "global.json",
            r#"{"sdk": {"version": "8.0.404", "rollForward": "latestFeature"}}"#,
        );

        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin
            .unpin_version(UnpinVersionInput {
                dir: plugin.tool.to_virtual_path(sandbox.path()),
                ..Default::default()
            })
            .await;

        assert!(output.unpinned);

        let content = read_global(&sandbox);

        assert!(!content.contains("8.0.404"));
        assert!(content.contains("rollForward"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unpinning_an_unpinned_file_changes_nothing() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("global.json", r#"{"sdk": {"rollForward": "disable"}}"#);

        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin
            .unpin_version(UnpinVersionInput {
                dir: plugin.tool.to_virtual_path(sandbox.path()),
                ..Default::default()
            })
            .await;

        assert!(!output.unpinned);
        assert!(output.version.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn exports_dotnet_root_on_activation() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin
            .activate_environment(ActivateEnvironmentInput {
                context: PluginContext {
                    tool_dir: plugin.tool.to_virtual_path(sandbox.path()),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        // The muxer does not need this, but MSBuild and the SDK resolvers do.
        assert!(output.env.contains_key("DOTNET_ROOT"));
    }
}
