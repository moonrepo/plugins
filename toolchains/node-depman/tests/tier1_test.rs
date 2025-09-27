use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;

mod node_depman_toolchain_tier1 {
    use super::*;

    mod register_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_npm() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("npm").await;

            let output = plugin
                .register_toolchain(RegisterToolchainInput { id: Id::raw("npm") })
                .await;

            assert_eq!(output.name, "npm");
            assert_eq!(output.config_file_globs, [".npmrc"]);
            assert_eq!(output.manifest_file_names, ["package.json"]);
            assert_eq!(
                output.lock_file_names,
                ["package-lock.json", "npm-shrinkwrap.json"]
            );
            assert_eq!(output.exe_names, ["npm", "npx"]);
            assert_eq!(output.vendor_dir_name.unwrap(), "node_modules");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_pnpm() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("pnpm").await;

            let output = plugin
                .register_toolchain(RegisterToolchainInput {
                    id: Id::raw("pnpm"),
                })
                .await;

            assert_eq!(output.name, "pnpm");
            assert_eq!(
                output.config_file_globs,
                [".npmrc", "pnpm-workspace.yaml", ".pnpmfile.*"]
            );
            assert_eq!(output.manifest_file_names, ["package.json"]);
            assert_eq!(output.lock_file_names, ["pnpm-lock.yaml"]);
            assert_eq!(output.exe_names, ["pnpm", "pnpx"]);
            assert_eq!(output.vendor_dir_name.unwrap(), "node_modules");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_yarn() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("yarn").await;

            let output = plugin
                .register_toolchain(RegisterToolchainInput {
                    id: Id::raw("yarn"),
                })
                .await;

            assert_eq!(output.name, "yarn");
            assert_eq!(output.config_file_globs, [".npmrc", ".yarnrc.*"]);
            assert_eq!(output.manifest_file_names, ["package.json"]);
            assert_eq!(output.lock_file_names, ["yarn.lock"]);
            assert_eq!(output.exe_names, ["yarn", "yarnpkg"]);
            assert_eq!(output.vendor_dir_name.unwrap(), "node_modules");
        }
    }
}
