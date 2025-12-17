use proto_pdk_test_utils::*;

mod moon_tool {
    use super::*;

    generate_download_install_tests!("moon-test", "1.30.0");

    mod canary {
        use super::*;

        generate_download_install_tests!("moon-test", "canary");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_for_moon_v2() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("2.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("moon_cli-aarch64-unknown-linux-gnu".into()),
                checksum_name: Some("moon_cli-aarch64-unknown-linux-gnu.tar.xz.sha256".into()),
                checksum_url: Some(
                    "https://github.com/moonrepo/moon/releases/download/v2.0.0/moon_cli-aarch64-unknown-linux-gnu.tar.xz.sha256".into()
                ),
                download_name: Some("moon_cli-aarch64-unknown-linux-gnu.tar.xz".into()),
                download_url: "https://github.com/moonrepo/moon/releases/download/v2.0.0/moon_cli-aarch64-unknown-linux-gnu.tar.xz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
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
                download_name: Some("moon-aarch64-unknown-linux-gnu".into()),
                download_url:
                    "https://github.com/moonrepo/moon/releases/download/v1.41.0/moon-aarch64-unknown-linux-gnu".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
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
                download_name: Some("moon-x86_64-unknown-linux-gnu".into()),
                download_url:
                    "https://github.com/moonrepo/moon/releases/download/v1.2.0/moon-x86_64-unknown-linux-gnu".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_for_moon_v2() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("2.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("moon_cli-aarch64-apple-darwin".into()),
                checksum_name: Some("moon_cli-aarch64-apple-darwin.tar.xz.sha256".into()),
                checksum_url: Some(
                    "https://github.com/moonrepo/moon/releases/download/v2.0.0/moon_cli-aarch64-apple-darwin.tar.xz.sha256".into()
                ),
                download_name: Some("moon_cli-aarch64-apple-darwin.tar.xz".into()),
                download_url: "https://github.com/moonrepo/moon/releases/download/v2.0.0/moon_cli-aarch64-apple-darwin.tar.xz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
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
                download_name: Some("moon-aarch64-apple-darwin".into()),
                download_url: "https://github.com/moonrepo/moon/releases/download/v1.2.0/moon-aarch64-apple-darwin"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
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
                download_name: Some("moon-x86_64-apple-darwin".into()),
                download_url: "https://github.com/moonrepo/moon/releases/download/v1.2.0/moon-x86_64-apple-darwin"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_for_moon_v2() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("2.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("moon_cli-x86_64-pc-windows-msvc".into()),
                checksum_name: Some("moon_cli-x86_64-pc-windows-msvc.zip.sha256".into()),
                checksum_url: Some(
                    "https://github.com/moonrepo/moon/releases/download/v2.0.0/moon_cli-x86_64-pc-windows-msvc.zip.sha256".into()
                ),
                download_name: Some("moon_cli-x86_64-pc-windows-msvc.zip".into()),
                download_url: "https://github.com/moonrepo/moon/releases/download/v2.0.0/moon_cli-x86_64-pc-windows-msvc.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported architecture arm64 for windows.")]
    async fn doesnt_support_windows_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
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
                download_name: Some("moon-aarch64-pc-windows-msvc.exe".into()),
                download_url: "https://github.com/moonrepo/moon/releases/download/v1.2.0/moon-aarch64-pc-windows-msvc.exe"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
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
                download_name: Some("moon-x86_64-pc-windows-msvc.exe".into()),
                download_url: "https://github.com/moonrepo/moon/releases/download/v1.2.0/moon-x86_64-pc-windows-msvc.exe"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
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
                .get("moon")
                .unwrap()
                .exe_path,
            Some("moon-test".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
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
                .get("moon")
                .unwrap()
                .exe_path,
            Some("moon-test.exe".into())
        );
    }
}
