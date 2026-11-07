use proto_pdk_test_utils::*;

mod proto_tool {
    use super::*;

    generate_download_install_tests!("proto-test", "0.63.2");

    // File names included the version, and checksums weren't published
    mod before_v0_8_3 {
        use super::*;

        generate_download_install_tests!("proto-test", "0.8.2");
    }

    async fn download(os: HostOS, arch: HostArch, version: &str) -> DownloadPrebuiltOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("proto-test", |config| {
                config.host(os, arch);
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse(version).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        assert_eq!(
            download(HostOS::Linux, HostArch::Arm64, "0.63.2").await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("proto_cli-aarch64-unknown-linux-gnu".into()),
                checksum_name: Some("proto_cli-aarch64-unknown-linux-gnu.tar.xz.sha256".into()),
                checksum_url: Some(
                    "https://github.com/moonrepo/proto/releases/download/v0.63.2/proto_cli-aarch64-unknown-linux-gnu.tar.xz.sha256".into()
                ),
                download_name: Some("proto_cli-aarch64-unknown-linux-gnu.tar.xz".into()),
                download_url: "https://github.com/moonrepo/proto/releases/download/v0.63.2/proto_cli-aarch64-unknown-linux-gnu.tar.xz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        assert_eq!(
            download(HostOS::Windows, HostArch::X64, "0.63.2").await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("proto_cli-x86_64-pc-windows-msvc".into()),
                checksum_name: Some("proto_cli-x86_64-pc-windows-msvc.zip.sha256".into()),
                checksum_url: Some(
                    "https://github.com/moonrepo/proto/releases/download/v0.63.2/proto_cli-x86_64-pc-windows-msvc.zip.sha256".into()
                ),
                download_name: Some("proto_cli-x86_64-pc-windows-msvc.zip".into()),
                download_url: "https://github.com/moonrepo/proto/releases/download/v0.63.2/proto_cli-x86_64-pc-windows-msvc.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported architecture arm64 for windows.")]
    async fn doesnt_support_windows_arm64() {
        download(HostOS::Windows, HostArch::Arm64, "0.63.2").await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn includes_version_in_file_name_before_v0_8_3() {
        assert_eq!(
            download(HostOS::MacOS, HostArch::Arm64, "0.8.2").await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("proto_cli-v0.8.2-aarch64-apple-darwin".into()),
                download_name: Some("proto_cli-v0.8.2-aarch64-apple-darwin.tar.xz".into()),
                download_url: "https://github.com/moonrepo/proto/releases/download/v0.8.2/proto_cli-v0.8.2-aarch64-apple-darwin.tar.xz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_checksum_before_v0_14_2() {
        assert_eq!(
            download(HostOS::MacOS, HostArch::Arm64, "0.14.1").await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("proto_cli-aarch64-apple-darwin".into()),
                download_name: Some("proto_cli-aarch64-apple-darwin.tar.xz".into()),
                download_url: "https://github.com/moonrepo/proto/releases/download/v0.14.1/proto_cli-aarch64-apple-darwin.tar.xz".into(),
                ..Default::default()
            }
        );

        let output = download(HostOS::MacOS, HostArch::Arm64, "0.14.2").await;

        assert_eq!(
            output.checksum_name,
            Some("proto_cli-aarch64-apple-darwin.tar.xz.sha256".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_proto_and_shim_without_bins_or_shims() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("proto-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput {
                context: PluginContext {
                    version: VersionSpec::parse("0.63.2").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        let proto = output.exes.get("proto").unwrap();
        let shim = output.exes.get("proto-shim").unwrap();

        assert!(proto.primary && proto.no_bin && proto.no_shim);
        assert_eq!(proto.exe_path, Some("proto.exe".into()));
        assert!(!shim.primary && shim.no_bin && shim.no_shim);
        assert_eq!(shim.exe_path, Some("proto-shim.exe".into()));
    }
}
