use proto_pdk_test_utils::*;

mod go_tool {
    use super::*;

    generate_download_install_tests!("go-test", "1.21.0");

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
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
                archive_prefix: Some("go".into()),
                checksum_url: Some(
                    "https://dl.google.com/go/go1.2.linux-arm64.tar.gz.sha256".into()
                ),
                download_name: Some("go1.2.linux-arm64.tar.gz".into()),
                download_url: "https://dl.google.com/go/go1.2.linux-arm64.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
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
                archive_prefix: Some("go".into()),
                checksum_url: Some(
                    "https://dl.google.com/go/go1.2.linux-amd64.tar.gz.sha256".into()
                ),
                download_name: Some("go1.2.linux-amd64.tar.gz".into()),
                download_url: "https://dl.google.com/go/go1.2.linux-amd64.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
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
                archive_prefix: Some("go".into()),
                checksum_url: Some(
                    "https://dl.google.com/go/go1.2.darwin-arm64.tar.gz.sha256".into()
                ),
                download_name: Some("go1.2.darwin-arm64.tar.gz".into()),
                download_url: "https://dl.google.com/go/go1.2.darwin-arm64.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
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
                archive_prefix: Some("go".into()),
                checksum_url: Some(
                    "https://dl.google.com/go/go1.2.darwin-amd64.tar.gz.sha256".into()
                ),
                download_name: Some("go1.2.darwin-amd64.tar.gz".into()),
                download_url: "https://dl.google.com/go/go1.2.darwin-amd64.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
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
                archive_prefix: Some("go".into()),
                checksum_url: Some(
                    "https://dl.google.com/go/go1.2.windows-amd64.zip.sha256".into()
                ),
                download_name: Some("go1.2.windows-amd64.zip".into()),
                download_url: "https://dl.google.com/go/go1.2.windows-amd64.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_freebsd_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
                config.host(HostOS::FreeBSD, HostArch::X64);
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
                archive_prefix: Some("go".into()),
                checksum_url: Some(
                    "https://dl.google.com/go/go1.2.freebsd-amd64.tar.gz.sha256".into()
                ),
                download_name: Some("go1.2.freebsd-amd64.tar.gz".into()),
                download_url: "https://dl.google.com/go/go1.2.freebsd-amd64.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                })
                .await
                .exes
                .get("go")
                .unwrap()
                .exe_path,
            Some("bin/go".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                })
                .await
                .exes
                .get("go")
                .unwrap()
                .exe_path,
            Some("bin/go.exe".into())
        );
    }
}
