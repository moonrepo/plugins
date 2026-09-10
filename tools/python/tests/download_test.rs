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

const DOWNLOAD_URL: &str = "https://github.com/astral-sh/python-build-standalone/releases/download";

mod python_tool {
    use super::*;

    generate_download_install_tests!("python-test", "3.10.0");

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64_gnu() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::X64;
                    host.libc = HostLibc::Gnu;
                });
            })
            .await;

        let file = "cpython-3.12.0+20231002-x86_64-unknown-linux-gnu-install_only.tar.gz";

        assert_eq!(
            plugin.download_prebuilt(create_input("3.12.0")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("python".into()),
                checksum_name: Some(format!("{file}.sha256")),
                checksum_url: Some(format!("{DOWNLOAD_URL}/20231002/{file}.sha256")),
                download_name: Some(file.into()),
                download_url: format!("{DOWNLOAD_URL}/20231002/{file}"),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64_musl() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::X64;
                    host.libc = HostLibc::Musl;
                });
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(create_input("3.12.0"))
                .await
                .download_name,
            Some("cpython-3.12.0+20231002-x86_64-unknown-linux-musl-install_only.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(create_input("3.12.0"))
                .await
                .download_name,
            Some("cpython-3.12.0+20231002-aarch64-apple-darwin-install_only.tar.gz".into())
        );
    }

    // The registry marks these artifacts with an `msvc` libc,
    // which isn't a libc that the host environment supports
    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(create_input("3.12.0"))
                .await
                .download_name,
            Some(
                "cpython-3.12.0+20231002-x86_64-pc-windows-msvc-shared-install_only.tar.gz".into()
            )
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_build_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        // The oldest of the two builds, instead of the latest
        assert_eq!(
            plugin
                .download_prebuilt(create_input("3.10.0+20211012"))
                .await
                .download_url,
            format!(
                "{DOWNLOAD_URL}/20211012/cpython-3.10.0-aarch64-apple-darwin-install_only-20211011T1926.tar.gz"
            )
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_prerelease() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        let file = "cpython-3.14.0b1+20250604-aarch64-apple-darwin-install_only.tar.gz";

        assert_eq!(
            plugin
                .download_prebuilt(create_input("3.14.0-beta.1"))
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("python".into()),
                checksum_name: Some(format!("{file}.sha256")),
                checksum_url: Some(format!("{DOWNLOAD_URL}/20250604/{file}.sha256")),
                download_name: Some(file.into()),
                download_url: format!("{DOWNLOAD_URL}/20250604/{file}"),
                ..Default::default()
            }
        );
    }

    // The release identifier is required for the download URL, so the default
    // build (which has no identifier) is looked up as the last entry instead
    #[tokio::test(flavor = "multi_thread")]
    async fn uses_default_build_when_unresolved() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        // Has builds 20211017 and 20211012
        assert_eq!(
            plugin
                .download_prebuilt(create_input("3.10.0"))
                .await
                .download_url,
            format!(
                "{DOWNLOAD_URL}/20211012/cpython-3.10.0-aarch64-apple-darwin-install_only-20211011T1926.tar.gz"
            )
        );
    }

    // Older releases have no checksums file
    #[tokio::test(flavor = "multi_thread")]
    async fn supports_releases_without_checksums() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        let output = plugin.download_prebuilt(create_input("3.10.0")).await;

        assert_eq!(output.checksum_name, None);
        assert_eq!(output.checksum_url, None);
        assert_eq!(output.archive_prefix, Some("python".into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "No pre-built available for 3.12.0 on linux-s390x")]
    async fn errors_for_unsupported_arch() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::S390x;
                    host.libc = HostLibc::Musl;
                });
            })
            .await;

        plugin.download_prebuilt(create_input("3.12.0")).await;
    }

    // The registry has no release, so the request itself fails
    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "releases/python/3.0.0 (404)")]
    async fn errors_for_unknown_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        plugin.download_prebuilt(create_input("3.0.0")).await;
    }
}
