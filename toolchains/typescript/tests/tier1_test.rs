use moon_pdk_api::DefineToolchainConfigOutput;
use moon_pdk_test_utils::create_empty_moon_sandbox;

mod typescript_toolchain_tier1 {
    use super::*;

    mod define_toolchain_config {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_prune_project_references_setting() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("typescript").await;

            let DefineToolchainConfigOutput { schema } = plugin
                .plugin
                .call_func_with("define_toolchain_config", ())
                .await
                .unwrap();
            let schema = serde_json::to_value(schema).unwrap();
            let schema = schema.as_object().unwrap();
            let ty = schema.get("ty").unwrap().as_object().unwrap();
            let fields = ty.get("fields").unwrap().as_object().unwrap();

            assert!(fields.contains_key("pruneProjectReferences"));
        }
    }
}
