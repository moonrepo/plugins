use proto_pdk_test_utils::*;

mod bun_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn puts_the_tool_dir_on_path() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;
        let tool_dir = sandbox.path().join(".proto/tools/bun-test/1.4.0");

        let output = plugin
            .activate_environment(ActivateEnvironmentInput {
                context: PluginContext {
                    tool_dir: VirtualPath::new(&tool_dir),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        // Real paths in WASM only use `/`, even on Windows
        assert_eq!(
            output.paths,
            [std::path::PathBuf::from(
                tool_dir.to_string_lossy().replace('\\', "/")
            )]
        );
    }
}
