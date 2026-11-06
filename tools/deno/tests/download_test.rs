use proto_pdk_test_utils::*;
use std::env;

mod deno_tool {
    use super::*;

    generate_download_install_tests!("deno-test", "1.30.0");

    mod canary {
        use super::*;

        generate_download_install_tests!("deno-test", "canary");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.41.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("deno-aarch64-unknown-linux-gnu.zip".into()),
                download_url:
                    "https://github.com/denoland/deno/releases/download/v1.41.0/deno-aarch64-unknown-linux-gnu.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("deno-x86_64-unknown-linux-gnu.zip".into()),
                download_url:
                    "https://github.com/denoland/deno/releases/download/v1.2.0/deno-x86_64-unknown-linux-gnu.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("deno-aarch64-apple-darwin.zip".into()),
                download_url: "https://github.com/denoland/deno/releases/download/v1.2.0/deno-aarch64-apple-darwin.zip"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::MacOS, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("deno-x86_64-apple-darwin.zip".into()),
                download_url: "https://github.com/denoland/deno/releases/download/v1.2.0/deno-x86_64-apple-darwin.zip"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::Windows, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("deno-aarch64-pc-windows-msvc.zip".into()),
                download_url: "https://github.com/denoland/deno/releases/download/v1.2.0/deno-aarch64-pc-windows-msvc.zip"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("deno-x86_64-pc-windows-msvc.zip".into()),
                download_url: "https://github.com/denoland/deno/releases/download/v1.2.0/deno-x86_64-pc-windows-msvc.zip"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("deno")
                .unwrap()
                .exe_path,
            Some("deno".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("deno")
                .unwrap()
                .exe_path,
            Some("deno.exe".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_checksum_gte_v2() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("2.1.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("deno-x86_64-unknown-linux-gnu.zip".into()),
                download_url:
                    "https://github.com/denoland/deno/releases/download/v2.1.0/deno-x86_64-unknown-linux-gnu.zip".into(),
                checksum_url: Some("https://github.com/denoland/deno/releases/download/v2.1.0/deno-x86_64-unknown-linux-gnu.zip.sha256sum".into()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_checksum_for_canary() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("canary").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert!(
            output
                .download_url
                .starts_with("https://dl.deno.land/canary/")
        );
        assert_eq!(
            output.checksum_url,
            Some(format!("{}.sha256sum", output.download_url))
        );
    }

    mod globals {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_install_root_bin_dir() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("deno-test").await;

            unsafe { env::set_var("DENO_INSTALL_ROOT", "/opt/deno") };

            let output = plugin
                .locate_executables(LocateExecutablesInput::default())
                .await;

            unsafe { env::remove_var("DENO_INSTALL_ROOT") };

            assert_eq!(
                output.globals_lookup_dirs,
                [
                    "$DENO_INSTALL_ROOT/bin",
                    "$DENO_HOME/bin",
                    "$HOME/.deno/bin"
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_install_root_as_is_when_a_bin_dir() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("deno-test").await;

            unsafe { env::set_var("DENO_INSTALL_ROOT", "/home/user/.deno/bin/") };

            let output = plugin
                .locate_executables(LocateExecutablesInput::default())
                .await;

            unsafe { env::remove_var("DENO_INSTALL_ROOT") };

            assert_eq!(
                output.globals_lookup_dirs,
                ["$DENO_INSTALL_ROOT", "$DENO_HOME/bin", "$HOME/.deno/bin"]
            );
        }
    }
}
