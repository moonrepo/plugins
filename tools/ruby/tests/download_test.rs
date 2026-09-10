use proto_pdk_test_utils::*;

fn create_input(version: &str) -> DownloadPrebuiltInput {
    DownloadPrebuiltInput {
        context: PluginContext {
            version: VersionSpec::parse(version).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}

mod ruby_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::X64;
                    host.libc = HostLibc::Gnu;
                });
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("3.4.5")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("ruby-3.4.5".into()),
                download_name: Some("ruby-3.4.5.x86_64_linux.tar.gz".into()),
                download_url:
                    "https://github.com/jdx/ruby/releases/download/3.4.5-1/ruby-3.4.5.x86_64_linux.tar.gz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::Arm64;
                    host.libc = HostLibc::Gnu;
                });
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("3.4.5")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("ruby-3.4.5".into()),
                download_name: Some("ruby-3.4.5.arm64_linux.tar.gz".into()),
                download_url:
                    "https://github.com/jdx/ruby/releases/download/3.4.5-1/ruby-3.4.5.arm64_linux.tar.gz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("3.4.5")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("ruby-3.4.5".into()),
                download_name: Some("ruby-3.4.5.macos.tar.gz".into()),
                download_url:
                    "https://github.com/jdx/ruby/releases/download/3.4.5-1/ruby-3.4.5.macos.tar.gz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_build_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("3.4.5+3")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("ruby-3.4.5".into()),
                download_name: Some("ruby-3.4.5.macos.tar.gz".into()),
                download_url:
                    "https://github.com/jdx/ruby/releases/download/3.4.5-3/ruby-3.4.5.macos.tar.gz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_prerelease() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("4.0.0-preview.2")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("ruby-4.0.0-preview2".into()),
                download_name: Some("ruby-4.0.0-preview2.macos.tar.gz".into()),
                download_url:
                    "https://github.com/jdx/ruby/releases/download/4.0.0-preview2-1/ruby-4.0.0-preview2.macos.tar.gz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_prerelease_with_build_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(create_input("4.0.0-preview.2+4"))
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("ruby-4.0.0-preview2".into()),
                download_name: Some("ruby-4.0.0-preview2.macos.tar.gz".into()),
                download_url:
                    "https://github.com/jdx/ruby/releases/download/4.0.0-preview2-4/ruby-4.0.0-preview2.macos.tar.gz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "No pre-built available for 3.4.5 on macos-x64")]
    async fn errors_for_unsupported_platform() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host(HostOS::MacOS, HostArch::X64);
            })
            .await;

        plugin.download_prebuilt(create_input("3.4.5")).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "No pre-built available for 3.4.5 on windows-x64")]
    async fn errors_for_windows() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        plugin.download_prebuilt(create_input("3.4.5")).await;
    }
}
