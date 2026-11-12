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
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm64));
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

    async fn download_for_arm_version(
        arm_version: Option<u8>,
        version: &str,
    ) -> DownloadPrebuiltOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::Arm;
                    host.libc = HostLibc::Gnu;
                    host.details = Some(HostDetails {
                        arm_version,
                        ..Default::default()
                    });
                });
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
    async fn supports_linux_armv6() {
        for (version, url) in [
            (
                "10.24.1",
                "https://nodejs.org/download/release/v10.24.1/node-v10.24.1-linux-armv6l.tar.xz",
            ),
            (
                "22.23.5",
                "https://unofficial-builds.nodejs.org/download/release/v22.23.5/node-v22.23.5-linux-armv6l.tar.xz",
            ),
        ] {
            let output = download_for_arm_version(Some(6), version).await;

            assert_eq!(output.download_url, url);
            assert_eq!(output.variant.as_deref(), Some("armv6l"));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_armv7_build_without_armv6() {
        for arm_version in [None, Some(7)] {
            let output = download_for_arm_version(arm_version, "24.0.0").await;

            assert_eq!(
                output.download_url,
                "https://nodejs.org/download/release/v24.0.0/node-v24.0.0-linux-armv7l.tar.xz"
            );
            assert_eq!(output.variant, None);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "No pre-built Node.js archive is available for ARMv6 from v23")]
    async fn errors_for_armv6_from_v23() {
        download_for_arm_version(Some(6), "23.0.0").await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm));
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
                config.host(HostPlatform::new(HostOS::Linux, HostArch::X64));
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
                config.host(HostPlatform::new(HostOS::Linux, HostArch::S390x));
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
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Powerpc64));
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
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::Arm64));
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
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::Arm64));
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
    async fn supports_linux_arm64_pre_v16() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("14.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .download_url,
            "https://nodejs.org/download/release/v14.0.0/node-v14.0.0-linux-arm64.tar.xz"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_arm64_pre_v20() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostPlatform::new(HostOS::Windows, HostArch::Arm64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("18.0.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .download_url,
            "https://nodejs.org/download/release/v18.0.0/node-v18.0.0-win-x64.zip"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_riscv64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Riscv64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("22.21.1").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("node-v22.21.1-linux-riscv64".into()),
                checksum_url: Some(
                    "https://unofficial-builds.nodejs.org/download/release/v22.21.1/SHASUMS256.txt"
                        .into()
                ),
                download_name: Some("node-v22.21.1-linux-riscv64.tar.xz".into()),
                download_url:
                    "https://unofficial-builds.nodejs.org/download/release/v22.21.1/node-v22.21.1-linux-riscv64.tar.xz"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_loong64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::LongArm64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("22.21.1").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .download_url,
            "https://unofficial-builds.nodejs.org/download/release/v22.21.1/node-v22.21.1-linux-loong64.tar.xz"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "No pre-built Node.js archive is available for musl Linux on s390x.")]
    async fn doesnt_support_musl_on_other_archs() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host_environment(HostEnvironment {
                    arch: HostArch::S390x,
                    libc: HostLibc::Musl,
                    os: HostOS::Linux,
                    ..Default::default()
                });
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("20.0.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::X64));
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
                config.host(HostPlatform::new(HostOS::Windows, HostArch::Arm64));
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
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
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
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X86));
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
                    .host(HostPlatform::new(HostOS::Linux, HostArch::X64))
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
                    .host(HostPlatform::new(HostOS::Linux, HostArch::X64))
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
    async fn uses_official_host_for_musl_canary() {
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
                .starts_with("https://nodejs.org/download/nightly/v")
        );
        assert!(output.download_url.ends_with("-linux-x64-musl.tar.xz"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm64));
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
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
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
