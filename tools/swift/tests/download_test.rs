use proto_pdk_test_utils::*;

mod swift_tool {
    use super::*;
    use ::swift_tool::{LinuxPlatform, SwiftToolConfig};

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_all_linux_platforms() {
        let platforms = [
            (LinuxPlatform::AmazonLinux2, "amazonlinux2", "amazonlinux2"),
            (
                LinuxPlatform::AmazonLinux2023,
                "amazonlinux2023",
                "amazonlinux2023",
            ),
            (LinuxPlatform::Debian12, "debian12", "debian12"),
            (LinuxPlatform::Fedora39, "fedora39", "fedora39"),
            (LinuxPlatform::Fedora41, "fedora41", "fedora41"),
            (LinuxPlatform::RedHatUbi9, "ubi9", "ubi9"),
            (LinuxPlatform::Ubuntu2004, "ubuntu2004", "ubuntu20.04"),
            (LinuxPlatform::Ubuntu2204, "ubuntu2204", "ubuntu22.04"),
            (LinuxPlatform::Ubuntu2404, "ubuntu2404", "ubuntu24.04"),
        ];

        for (linux_platform, platform, archive_suffix) in platforms {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("swift-test", |config| {
                    config
                        .host(HostOS::Linux, HostArch::X64)
                        .tool_config(SwiftToolConfig {
                            linux_platform: linux_platform.clone(),
                            ..Default::default()
                        });
                })
                .await;
            let folder = "swift-6.1.2-RELEASE";
            let archive_prefix = format!("{folder}-{archive_suffix}");

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("6.1.2").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some(archive_prefix.clone()),
                    download_name: Some(format!("{archive_prefix}.tar.gz")),
                    download_url: format!(
                        "https://download.swift.org/swift-6.1.2-release/{platform}/{folder}/{archive_prefix}.tar.gz"
                    ),
                    ..Default::default()
                }
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("swift-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("6.1.2").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("swift-6.1.2-RELEASE-ubuntu24.04-aarch64".into()),
                download_name: Some("swift-6.1.2-RELEASE-ubuntu24.04-aarch64.tar.gz".into()),
                download_url:
                    "https://download.swift.org/swift-6.1.2-release/ubuntu2404-aarch64/swift-6.1.2-RELEASE/swift-6.1.2-RELEASE-ubuntu24.04-aarch64.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("swift-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("6.1.2").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("swift-6.1.2-RELEASE-ubuntu24.04".into()),
                download_name: Some("swift-6.1.2-RELEASE-ubuntu24.04.tar.gz".into()),
                download_url:
                    "https://download.swift.org/swift-6.1.2-release/ubuntu2404/swift-6.1.2-RELEASE/swift-6.1.2-RELEASE-ubuntu24.04.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bins() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("swift-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput {
                context: PluginContext {
                    version: VersionSpec::parse("6.1.2").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.exes.get("swift").unwrap().exe_path,
            Some("usr/bin/swift".into())
        );
        assert_eq!(
            output.exes.get("swiftc").unwrap().exe_path,
            Some("usr/bin/swiftc".into())
        );
        assert_eq!(
            output.exes.get("sourcekit-lsp").unwrap().exe_path,
            Some("usr/bin/sourcekit-lsp".into())
        );
    }
}
