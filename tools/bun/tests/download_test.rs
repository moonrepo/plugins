use proto_pdk_test_utils::*;

mod bun_tool {
    use super::*;

    generate_download_install_tests!("bun-test", "1.1.0");

    mod canary {
        use super::*;

        generate_download_install_tests!("bun-test", "canary");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: ToolContext {
                    version: VersionSpec::parse("1.2.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await,
        DownloadPrebuiltOutput {
            archive_prefix: Some("bun-linux-aarch64".into()),
            checksum_url: Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            ),
            download_name: Some("bun-linux-aarch64.zip".into()),
            download_url:
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-linux-aarch64.zip"
                    .into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let result = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: ToolContext {
                    version: VersionSpec::parse("1.2.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            result.checksum_url,
            Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            )
        );

        // This is different between boxes in CI, so impossible to capture!
        // assert_eq!(result.archive_prefix, Some("bun-linux-x64".into()));
        // assert_eq!(result.download_name, Some("bun-linux-x64.zip".into()));
        assert!(
        result.download_url ==
        "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-linux-x64.zip" || result.download_url ==
        "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-linux-x64-baseline.zip"
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: ToolContext {
                    version: VersionSpec::parse("1.2.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await,
        DownloadPrebuiltOutput {
            archive_prefix: Some("bun-darwin-aarch64".into()),
            checksum_url: Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            ),
            download_name: Some("bun-darwin-aarch64.zip".into()),
            download_url:
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-darwin-aarch64.zip"
                    .into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostOS::MacOS, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: ToolContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("bun-darwin-x64".into()),
                checksum_url: Some(
                    "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt"
                        .into()
                ),
                download_name: Some("bun-darwin-x64.zip".into()),
                download_url:
                    "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-darwin-x64.zip"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: ToolContext {
                    version: VersionSpec::parse("1.2.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await,
        DownloadPrebuiltOutput {
            archive_prefix: Some("bun-windows-x64".into()),
            checksum_url: Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            ),
            download_name: Some("bun-windows-x64.zip".into()),
            download_url:
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-windows-x64.zip"
                    .into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    }
                })
                .await
                .exes
                .get("bun")
                .unwrap()
                .exe_path,
            Some("bun".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    }
                })
                .await
                .exes
                .get("bun")
                .unwrap()
                .exe_path,
            Some("bun.exe".into())
        );
    }
}
