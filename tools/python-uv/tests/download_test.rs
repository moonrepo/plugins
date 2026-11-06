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
                    context: PluginContext {
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
                download_url: "https://github.com/astral-sh/uv/releases/download/1.41.0/uv-aarch64-unknown-linux-gnu.tar.gz".into(),
                ..Default::default()
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
                    context: PluginContext {
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
                download_url: "https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-unknown-linux-gnu.tar.gz".into(),
                ..Default::default()
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
                    context: PluginContext {
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
                ..Default::default()
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
                    context: PluginContext {
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
                download_url: "https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-apple-darwin.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported architecture longarm64 for linux.")]
    async fn doesnt_support_linux_loongarch64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
                config.host(HostOS::Linux, HostArch::LongArm64);
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.2.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
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
                archive_prefix: None,
                checksum_name: Some("uv-aarch64-pc-windows-msvc.zip.sha256".into()),
                checksum_public_key: None,
                checksum_url: Some("https://github.com/astral-sh/uv/releases/download/1.2.0/uv-aarch64-pc-windows-msvc.zip.sha256".into()),
                download_name: Some("uv-aarch64-pc-windows-msvc.zip".into()),
                download_url: "https://github.com/astral-sh/uv/releases/download/1.2.0/uv-aarch64-pc-windows-msvc.zip".into(),
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
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: None,
                checksum_name: Some("uv-x86_64-pc-windows-msvc.zip.sha256".into()),
                checksum_public_key: None,
                checksum_url: Some("https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-pc-windows-msvc.zip.sha256".into()),
                download_name: Some("uv-x86_64-pc-windows-msvc.zip".into()),
                download_url: "https://github.com/astral-sh/uv/releases/download/1.2.0/uv-x86_64-pc-windows-msvc.zip".into(),
                ..Default::default()
            }
        );
    }

    async fn download_url(os: HostOS, arch: HostArch, libc: HostLibc) -> String {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("uv-test", |config| {
                config.host_with(|host| {
                    host.os = os;
                    host.arch = arch;
                    host.libc = libc;
                });
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("0.9.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
            .download_url
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_more_linux_and_windows_arches() {
        for (os, arch, libc, file) in [
            (
                HostOS::Linux,
                HostArch::X64,
                HostLibc::Musl,
                "x86_64-unknown-linux-musl.tar.gz",
            ),
            (
                HostOS::Linux,
                HostArch::X86,
                HostLibc::Gnu,
                "i686-unknown-linux-gnu.tar.gz",
            ),
            (
                HostOS::Linux,
                HostArch::Arm,
                HostLibc::Gnu,
                "arm-unknown-linux-musleabihf.tar.gz",
            ),
            (
                HostOS::Linux,
                HostArch::Powerpc64,
                HostLibc::Gnu,
                "powerpc64le-unknown-linux-gnu.tar.gz",
            ),
            (
                HostOS::Linux,
                HostArch::S390x,
                HostLibc::Gnu,
                "s390x-unknown-linux-gnu.tar.gz",
            ),
            (
                HostOS::Linux,
                HostArch::Riscv64,
                HostLibc::Gnu,
                "riscv64gc-unknown-linux-gnu.tar.gz",
            ),
            (
                HostOS::Windows,
                HostArch::X86,
                HostLibc::Unknown,
                "i686-pc-windows-msvc.zip",
            ),
        ] {
            assert_eq!(
                download_url(os, arch, libc).await,
                format!("https://github.com/astral-sh/uv/releases/download/0.9.0/uv-{file}")
            );
        }
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
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
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
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
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
