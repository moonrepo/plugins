use proto_pdk_test_utils::*;

mod go_tool {
    use super::*;

    generate_download_install_tests!("go-test", "1.21.0");

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm64));
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
                config.host(HostPlatform::new(HostOS::Linux, HostArch::X64));
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
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::Arm64));
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
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::X64));
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
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
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
                config.host(HostPlatform::new(HostOS::FreeBSD, HostArch::X64));
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
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm64));
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
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
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
                .get("go")
                .unwrap()
                .exe_path,
            Some("bin/go.exe".into())
        );
    }

    async fn download_url(os: HostOS, arch: HostArch, version: &str) -> String {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("go-test", |config| {
                config.host(HostPlatform::new(os, arch));
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
            .download_url
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn formats_prereleases_without_patch() {
        assert_eq!(
            download_url(HostOS::Linux, HostArch::X64, "1.27.0-rc1").await,
            "https://dl.google.com/go/go1.27rc1.linux-amd64.tar.gz"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_more_linux_and_freebsd_arches() {
        for (os, arch, file) in [
            (HostOS::Linux, HostArch::Riscv64, "linux-riscv64"),
            (HostOS::Linux, HostArch::Powerpc64, "linux-ppc64le"),
            (HostOS::Linux, HostArch::LongArm64, "linux-loong64"),
            (HostOS::FreeBSD, HostArch::Arm64, "freebsd-arm64"),
        ] {
            assert_eq!(
                download_url(os, arch, "1.26.2").await,
                format!("https://dl.google.com/go/go1.26.2.{file}.tar.gz")
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_look_for_globals_in_goroot() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;

        assert_eq!(
            output.globals_lookup_dirs,
            ["$GOBIN", "$GOPATH/bin", "$HOME/go/bin"]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_goroot_to_the_tool_dir() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;
        let tool_dir = sandbox.path().join(".proto/tools/go-test/1.26.2");

        let output = plugin
            .activate_environment(ActivateEnvironmentInput {
                context: PluginContext {
                    tool_dir: VirtualPath::new(&tool_dir),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        // Real paths in WASM only use `/`, even on Windows
        assert_eq!(
            output.env.get("GOROOT").unwrap(),
            &tool_dir.to_string_lossy().replace('\\', "/")
        );
    }
}
