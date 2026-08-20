use moon_config::UnresolvedVersionSpec;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod dotnet_toolchain_tier3 {
    use super::*;

    mod setup_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn no_configured_version_is_a_noop() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("dotnet").await;

            let output = plugin
                .setup_toolchain(SetupToolchainInput {
                    configured_version: None,
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(!output.installed);
            assert!(output.operations.is_empty());
            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn exact_version_already_installed_skips_without_network() {
            let sandbox = create_empty_moon_sandbox();
            // A pre-existing SDK layout at the default `~/.dotnet` root.
            // Anything past the short-circuit would hit the network and
            // fail the test, so completing cleanly proves the skip.
            sandbox.create_file(".home/.dotnet/sdk/8.0.404/marker", "");

            let plugin = sandbox.create_toolchain("dotnet").await;

            let output = plugin
                .setup_toolchain(SetupToolchainInput {
                    configured_version: Some(UnresolvedVersionSpec::parse("8.0.404").unwrap()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert!(!output.installed);
            assert!(output.operations.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Unsupported .NET version")]
        async fn unsupported_version_spec_errors() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("dotnet").await;

            plugin
                .setup_toolchain(SetupToolchainInput {
                    configured_version: Some(UnresolvedVersionSpec::parse("canary").unwrap()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;
        }

        // Downloads and runs the real dotnet-install script, fetching a full
        // ~200 MB SDK, so it stays out of the default run. On demand:
        //   cargo nextest run -p dotnet_toolchain --no-default-features         //     --run-ignored=only full_sdk_install
        #[tokio::test(flavor = "multi_thread")]
        #[ignore = "downloads a full .NET SDK from the network"]
        async fn full_sdk_install() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("dotnet").await;

            let root = sandbox.path().join(".home/.dotnet");

            let output = plugin
                .setup_toolchain(SetupToolchainInput {
                    configured_version: Some(UnresolvedVersionSpec::parse("8.0").unwrap()),
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.operations.len(), 1);

            let exe = if cfg!(windows) {
                "dotnet.exe"
            } else {
                "dotnet"
            };
            assert!(root.join(exe).exists());
            assert!(root.join("sdk").exists());
        }
    }
}
