use proto_pdk_test_utils::*;

mod rust_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_create_global_shims() {
        let sandbox = create_empty_proto_sandbox();
        let mut plugin = sandbox.create_plugin("rust-test").await;

        plugin.tool.generate_shims(false).await.unwrap();

        assert!(!sandbox.path().join(".proto/bin/rustc").exists());
        assert!(!sandbox.path().join(".proto/bin/cargo").exists());
        assert!(!sandbox.path().join(".proto/shims/rustc").exists());
        assert!(!sandbox.path().join(".proto/shims/cargo").exists());
    }
}
