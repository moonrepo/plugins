use proto_pdk_test_utils::*;

mod node_tool {
    use super::*;
    use ::node_tool::NodeToolConfig;

    generate_download_install_tests!("node-test", "18.0.0");

    // Doesn't provide macos x64 builds
    #[cfg(not(target_os = "macos"))]
    mod canary {
        use super::*;

        generate_download_install_tests!("node-test", "canary");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-arm64".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-linux-arm64.tar.xz".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-linux-arm64.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-armv7l".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-linux-armv7l.tar.xz".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-linux-armv7l.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64_musl() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host_environment(HostEnvironment {
                    arch: HostArch::Arm64,
                    libc: HostLibc::Musl,
                    os: HostOS::Linux,
                    ..Default::default()
                });
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-arm64-musl".into()),
                checksum_url: Some(
                    "https://unofficial-builds.nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-linux-arm64-musl.tar.xz".into()),
                download_url:
                    "https://unofficial-builds.nodejs.org/download/release/v20.0.0/node-v20.0.0-linux-arm64-musl.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-x64".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-linux-x64.tar.xz".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-linux-x64.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64_musl() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host_environment(HostEnvironment {
                    arch: HostArch::X64,
                    libc: HostLibc::Musl,
                    os: HostOS::Linux,
                    ..Default::default()
                });
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-x64-musl".into()),
                checksum_url: Some(
                    "https://unofficial-builds.nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-linux-x64-musl.tar.xz".into()),
                download_url:
                    "https://unofficial-builds.nodejs.org/download/release/v20.0.0/node-v20.0.0-linux-x64-musl.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_s390x() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::Linux, HostArch::S390x);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-s390x".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-linux-s390x.tar.xz".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-linux-s390x.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_ppc64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::Linux, HostArch::Powerpc64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-ppc64le".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-linux-ppc64le.tar.xz".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-linux-ppc64le.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-darwin-arm64".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-darwin-arm64.tar.xz".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-darwin-arm64.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64_pre_v16() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("12.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v12.0.0-darwin-x64".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v12.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v12.0.0-darwin-x64.tar.xz".into()),
                download_url:
                    "https://nodejs.org/download/release/v12.0.0/node-v12.0.0-darwin-x64.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::MacOS, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-darwin-x64".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-darwin-x64.tar.xz".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-darwin-x64.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::Windows, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-win-arm64".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-win-arm64.zip".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-win-arm64.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-win-x64".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-win-x64.zip".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-win-x64.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x86() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostOS::Windows, HostArch::X86);
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-win-x86".into()),
                checksum_url: Some(
                    "https://nodejs.org/download/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-win-x86.zip".into()),
                download_url:
                    "https://nodejs.org/download/release/v20.0.0/node-v20.0.0-win-x86.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_custom_dist_url() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config
                    .host(HostOS::Linux, HostArch::X64)
                    .tool_config(NodeToolConfig {
                        dist_url: "https://mirror.com/node/{channel}/v{version}/{file}".into(),
                        ..Default::default()
                    });
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-x64".into()),
                checksum_url: Some("https://mirror.com/node/release/v20.0.0/SHASUMS256.txt".into()),
                download_name: Some("node-v20.0.0-linux-x64.tar.xz".into()),
                download_url:
                    "https://mirror.com/node/release/v20.0.0/node-v20.0.0-linux-x64.tar.xz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_custom_dist_url_unofficial() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config
                    .host_environment(HostEnvironment {
                        arch: HostArch::X64,
                        libc: HostLibc::Musl,
                        os: HostOS::Linux,
                        ..Default::default()
                    })
                    .tool_config(NodeToolConfig {
                        dist_url: "https://mirror.com/node/{channel}/v{version}/{file}".into(),
                        dist_url_unofficial:
                            "https://mirror.com/unofficial/{channel}/v{version}/{file}".into(),
                        ..Default::default()
                    });
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v20.0.0-linux-x64-musl".into()),
                checksum_url: Some(
                    "https://mirror.com/unofficial/release/v20.0.0/SHASUMS256.txt".into()
                ),
                download_name: Some("node-v20.0.0-linux-x64-musl.tar.xz".into()),
                download_url:
                    "https://mirror.com/unofficial/release/v20.0.0/node-v20.0.0-linux-x64-musl.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_nightly_channel_for_canary() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config
                    .host(HostOS::Linux, HostArch::X64)
                    .tool_config(NodeToolConfig {
                        dist_url: "https://mirror.com/node/{channel}/v{version}/{file}".into(),
                        ..Default::default()
                    });
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("canary").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert!(
            output
                .download_url
                .starts_with("https://mirror.com/node/nightly/v")
        );
        assert!(
            output
                .checksum_url
                .unwrap()
                .starts_with("https://mirror.com/node/nightly/v")
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
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("node")
                .unwrap()
                .exe_path,
            Some("bin/node".into())
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
                    context: PluginContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("node")
                .unwrap()
                .exe_path,
            Some("node.exe".into())
        );
    }
}
