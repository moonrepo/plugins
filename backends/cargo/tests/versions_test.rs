use extism_pdk::json::json;
use proto_pdk_test_utils::*;

mod npm_backend {
    use super::*;

    generate_resolve_versions_tests!("npm:typescript", {
        "5.7" => "5.7.3",
        "5.9.2" => "5.9.2",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_via_npm() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("npm:typescript").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
        assert!(!output.aliases.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_via_bun() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("npm:typescript", |cfg| {
                cfg.backend_config(json!({ "bun": true }));
            })
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
        assert!(!output.aliases.is_empty());
    }
}
