use proto_pdk_test_utils::*;

mod pkl_tool {
    use super::*;

    generate_download_install_tests!("pkl-test", "0.30.0");

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "Pkl does not publish canary builds")]
    async fn doesnt_support_canary() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("pkl-test").await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::Canary,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::X64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("0.30.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("pkl-linux-amd64".into()),
                download_url:
                    "https://github.com/apple/pkl/releases/download/0.30.0/pkl-linux-amd64".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("0.30.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("pkl-linux-aarch64".into()),
                download_url:
                    "https://github.com/apple/pkl/releases/download/0.30.0/pkl-linux-aarch64".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64_musl() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(
                    HostPlatform::new(HostOS::Linux, HostArch::X64).with_libc(HostLibc::Musl),
                );
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("0.30.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("pkl-alpine-linux-amd64".into()),
                download_url:
                    "https://github.com/apple/pkl/releases/download/0.30.0/pkl-alpine-linux-amd64"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::Arm64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("0.33.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("pkl-macos-aarch64".into()),
                download_url:
                    "https://github.com/apple/pkl/releases/download/0.33.0/pkl-macos-aarch64".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64_before_v0_33() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::X64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("0.32.2").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("pkl-macos-amd64".into()),
                download_url:
                    "https://github.com/apple/pkl/releases/download/0.32.2/pkl-macos-amd64".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64_from_v0_26() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("0.26.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                download_name: Some("pkl-windows-amd64.exe".into()),
                download_url:
                    "https://github.com/apple/pkl/releases/download/0.26.0/pkl-windows-amd64.exe"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported architecture arm64 for linux.")]
    async fn doesnt_support_linux_arm64_musl() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(
                    HostPlatform::new(HostOS::Linux, HostArch::Arm64).with_libc(HostLibc::Musl),
                );
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("0.30.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported architecture x64 for macos.")]
    async fn doesnt_support_macos_x64_from_v0_33() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::X64));
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("0.33.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported architecture x64 for windows.")]
    async fn doesnt_support_windows_before_v0_26() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("0.25.3").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm64));
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: PluginContext {
                        version: VersionSpec::parse("0.30.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("pkl")
                .unwrap()
                .exe_path,
            Some("pkl-test".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("pkl-test", |config| {
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: PluginContext {
                        version: VersionSpec::parse("0.30.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("pkl")
                .unwrap()
                .exe_path,
            Some("pkl-test.exe".into())
        );
    }
}
