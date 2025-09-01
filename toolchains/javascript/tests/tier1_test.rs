use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use starbase_utils::json::JsonValue;

mod javascript_toolchain_tier1 {
    use super::*;

    mod initialize_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_bun_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("bun.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("bun".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_bun_via_lockfile2() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("bun.lockb", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("bun".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_bun_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package.json", r#"{ "packageManager": "bun" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("bun".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_deno_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("deno.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("deno".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_npm_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package-lock.json", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("npm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_npm_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package.json", r#"{ "packageManager": "npm@10" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("npm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_pnpm_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("pnpm-lock.yaml", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("pnpm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_pnpm_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package.json", r#"{ "packageManager": "pnpm@10.0.0" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("pnpm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_yarn_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("yarn.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("yarn".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_yarn_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{ "packageManager": "yarn@10.0.0+abcd" }"#,
            );

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("yarn".into())
            );
        }
    }
}
