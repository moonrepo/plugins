use proto_pdk_test_utils::*;

mod python_uv_tool {
    use super::*;

    generate_download_install_tests!("uv-test", "0.5.21");

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: ToolContext {
                        version: VersionSpec::parse("1.41.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("uv-aarch64-unknown-linux-gnu".into()),
                checksum_name: Some("uv-aarch64-unknown-linux-gnu.tar.gz.sha256".into()),
                checksum_public_key: None,
                checksum_url: Some("https://github.com/astral-sh/uv/releases/download/1.41.0/uv-aarch64-unknown-linux-gnu.tar.gz.sha256".into()),
                download_name: Some("uv-aarch64-unknown-linux-gnu.tar.gz".into()),
                download_url: "https://github.com/astral-sh/uv/releases/download/1.41.0/uv-aarch64-unknown-linux-gnu.tar.gz".into()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
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
                archive_prefix: Some("uv-x86_64-unknown-linux-gnu".into()),
                checksum_name: Some("uv-x86_64-unknown-linux-gnu.tar.gz.sha256".into()),
                checksum_public_key: None,
                checksum_url: Some("https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-unknown-linux-gnu.tar.gz.sha256".into()),
                download_name: Some("uv-x86_64-unknown-linux-gnu.tar.gz".into()),
                download_url: "https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-unknown-linux-gnu.tar.gz".into()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
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
                archive_prefix: Some("uv-aarch64-apple-darwin".into()),
                checksum_name: Some("uv-aarch64-apple-darwin.tar.gz.sha256".into()), checksum_public_key: None,
                checksum_url: Some("https://github.com/astral-sh/uv/releases/download/1.2.0/uv-aarch64-apple-darwin.tar.gz.sha256".into()),
                download_name: Some("uv-aarch64-apple-darwin.tar.gz".into()),
                download_url: "https://github.com/astral-sh/uv/releases/download/1.2.0/uv-aarch64-apple-darwin.tar.gz".into(),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
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
                archive_prefix: Some("uv-x86_64-apple-darwin".into()),
                checksum_name: Some("uv-x86_64-apple-darwin.tar.gz.sha256".into()),
                checksum_public_key: None,
                checksum_url: Some("https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-apple-darwin.tar.gz.sha256".into()),
                download_name: Some("uv-x86_64-apple-darwin.tar.gz".into()),
                download_url: "https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-apple-darwin.tar.gz".into()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported architecture arm64 for windows.")]
    async fn doesnt_support_windows_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
                config.host(HostOS::Windows, HostArch::Arm64);
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
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
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
                archive_prefix: Some("uv-x86_64-pc-windows-msvc".into()),
                checksum_name: Some("uv-x86_64-pc-windows-msvc.zip.sha256".into()),
                checksum_public_key: None,
                checksum_url: Some("https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-pc-windows-msvc.zip.sha256".into()),
                download_name: Some("uv-x86_64-pc-windows-msvc.zip".into()),
                download_url: "https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-pc-windows-msvc.zip".into()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
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
                .get("uv")
                .unwrap()
                .exe_path,
            Some("uv".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
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
                .get("uv")
                .unwrap()
                .exe_path,
            Some("uv.exe".into())
        );
    }
}
