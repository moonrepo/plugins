use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;

mod node_depman_toolchain_tier2 {
    use super::*;

    mod define_requirements {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn requires_node() {
            for id in ["npm", "nub", "pnpm", "yarn"] {
                let sandbox = create_empty_moon_sandbox();
                let plugin = sandbox.create_toolchain(id).await;

                let output: DefineRequirementsOutput = plugin
                    .plugin
                    .call_func_with("define_requirements", DefineRequirementsInput::default())
                    .await
                    .unwrap();

                assert_eq!(output.requires, ["node"], "{id}");
                assert!(output.for_setup_toolchain, "{id}");
            }
        }
    }
}
