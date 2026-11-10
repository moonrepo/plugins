use proto_pdk_test_utils::*;

mod bun_tool {
    use super::*;

    generate_download_install_tests!("bun-test", "1.1.0");

    mod canary {
        use super::*;

        generate_download_install_tests!("bun-test", "canary");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
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
            archive_prefix: Some("bun-linux-aarch64".into()),
            checksum_url: Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            ),
            download_name: Some("bun-linux-aarch64.zip".into()),
            download_url:
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-linux-aarch64.zip"
                    .into(),
            ..Default::default()
        }
    );
    }

    // AVX2 can't be detected for another platform, so the regular build is used
    #[tokio::test(flavor = "multi_thread")]
    async fn uses_regular_build_when_targeting_another_platform() {
        for (os, file) in [
            (HostOS::Linux, "bun-linux-x64.zip"),
            (HostOS::MacOS, "bun-darwin-x64.zip"),
        ] {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("bun-test", |config| {
                    config.host_with(|host| {
                        host.os = os;
                        host.arch = HostArch::X64;
                        host.libc = HostLibc::Gnu;
                        host.foreign = true;
                        host.details = None;
                    });
                })
                .await;

            let result = plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;

            assert_eq!(
                result.download_url,
                format!("https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/{file}")
            );
            assert_eq!(result.variant, None);
        }
    }

    // AVX2 is part of the x86-64-v3 level, which proto detects
    #[tokio::test(flavor = "multi_thread")]
    async fn uses_build_for_detected_x64_level() {
        for (os, level, file, variant) in [
            (
                HostOS::Linux,
                2,
                "bun-linux-x64-baseline.zip",
                Some("baseline"),
            ),
            (HostOS::Linux, 3, "bun-linux-x64.zip", None),
            (HostOS::Linux, 4, "bun-linux-x64.zip", None),
            (
                HostOS::MacOS,
                2,
                "bun-darwin-x64-baseline.zip",
                Some("baseline"),
            ),
            (HostOS::MacOS, 3, "bun-darwin-x64.zip", None),
        ] {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("bun-test", |config| {
                    config.host_with(|host| {
                        host.os = os;
                        host.arch = HostArch::X64;
                        host.libc = HostLibc::Gnu;
                        host.details = Some(HostDetails {
                            x64_level: Some(level),
                            ..Default::default()
                        });
                    });
                })
                .await;

            let result = plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;

            assert_eq!(
                result.download_url,
                format!("https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/{file}")
            );
            assert_eq!(result.variant.as_deref(), variant);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_baseline_build_for_musl_without_avx2() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::X64;
                    host.libc = HostLibc::Musl;
                    host.details = Some(HostDetails {
                        x64_level: Some(2),
                        ..Default::default()
                    });
                });
            })
            .await;

        let result = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.2.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            result.download_url,
            "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-linux-x64-musl-baseline.zip"
        );
        assert_eq!(result.variant.as_deref(), Some("baseline"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::X64));
            })
            .await;

        let result = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.2.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            result.checksum_url,
            Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            )
        );

        // This is different between boxes in CI, so impossible to capture!
        // assert_eq!(result.archive_prefix, Some("bun-linux-x64".into()));
        // assert_eq!(result.download_name, Some("bun-linux-x64.zip".into()));
        assert!(
            result.download_url
                == "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-linux-x64.zip"
                || result.download_url
                    == "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-linux-x64-baseline.zip"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64_musl() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::Arm64;
                    host.libc = HostLibc::Musl;
                });
            })
            .await;

        assert_eq!(
        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.4.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await,
        DownloadPrebuiltOutput {
            archive_prefix: Some("bun-linux-aarch64-musl".into()),
            checksum_url: Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.4.0/SHASUMS256.txt".into()
            ),
            download_name: Some("bun-linux-aarch64-musl.zip".into()),
            download_url:
                "https://github.com/oven-sh/bun/releases/download/bun-v1.4.0/bun-linux-aarch64-musl.zip"
                    .into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64_musl() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::X64;
                    host.libc = HostLibc::Musl;
                });
            })
            .await;

        let result = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.4.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        // The baseline suffix is different between boxes in CI!
        assert!(
            result.download_url
                == "https://github.com/oven-sh/bun/releases/download/bun-v1.4.0/bun-linux-x64-musl.zip"
                || result.download_url
                    == "https://github.com/oven-sh/bun/releases/download/bun-v1.4.0/bun-linux-x64-musl-baseline.zip"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "musl is only supported for Bun v1.1.35 and above.")]
    async fn doesnt_support_musl_before_v1135() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::Arm64;
                    host.libc = HostLibc::Musl;
                });
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.1.34").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
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
            archive_prefix: Some("bun-darwin-aarch64".into()),
            checksum_url: Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            ),
            download_name: Some("bun-darwin-aarch64.zip".into()),
            download_url:
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-darwin-aarch64.zip"
                    .into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::MacOS, HostArch::X64));
            })
            .await;

        let result = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.2.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            result.checksum_url,
            Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            )
        );

        // The baseline suffix depends on the host CPU (always baseline on Apple Silicon)
        assert!(
            result.download_url
                == "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-darwin-x64.zip"
                || result.download_url
                    == "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-darwin-x64-baseline.zip"
        );
        assert_eq!(
            result.archive_prefix.map(|prefix| format!("{prefix}.zip")),
            result.download_name
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_freebsd_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::FreeBSD, HostArch::Arm64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.3.14").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("bun-freebsd-aarch64".into()),
                checksum_url: Some(
                    "https://github.com/oven-sh/bun/releases/download/bun-v1.3.14/SHASUMS256.txt"
                        .into()
                ),
                download_name: Some("bun-freebsd-aarch64.zip".into()),
                download_url:
                    "https://github.com/oven-sh/bun/releases/download/bun-v1.3.14/bun-freebsd-aarch64.zip"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_freebsd_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::FreeBSD, HostArch::X64));
            })
            .await;

        assert_eq!(
            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.4.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("bun-freebsd-x64".into()),
                checksum_url: Some(
                    "https://github.com/oven-sh/bun/releases/download/bun-v1.4.0/SHASUMS256.txt"
                        .into()
                ),
                download_name: Some("bun-freebsd-x64.zip".into()),
                download_url:
                    "https://github.com/oven-sh/bun/releases/download/bun-v1.4.0/bun-freebsd-x64.zip"
                        .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported OS freebsd.")]
    async fn doesnt_support_freebsd_before_v1314() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::FreeBSD, HostArch::X64));
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.3.13").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
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
            archive_prefix: Some("bun-windows-x64".into()),
            checksum_url: Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/SHASUMS256.txt".into()
            ),
            download_name: Some("bun-windows-x64.zip".into()),
            download_url:
                "https://github.com/oven-sh/bun/releases/download/bun-v1.2.0/bun-windows-x64.zip"
                    .into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::Windows, HostArch::Arm64));
            })
            .await;

        assert_eq!(
        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.4.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await,
        DownloadPrebuiltOutput {
            archive_prefix: Some("bun-windows-aarch64".into()),
            checksum_url: Some(
                "https://github.com/oven-sh/bun/releases/download/bun-v1.4.0/SHASUMS256.txt".into()
            ),
            download_name: Some("bun-windows-aarch64.zip".into()),
            download_url:
                "https://github.com/oven-sh/bun/releases/download/bun-v1.4.0/bun-windows-aarch64.zip"
                    .into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported architecture arm64 for windows.")]
    async fn doesnt_support_windows_arm64_before_v1310() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::Windows, HostArch::Arm64));
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.3.9").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unsupported OS windows.")]
    async fn doesnt_support_windows_before_v11() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("bun-test", |config| {
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("1.0.30").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
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
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("bun")
                .unwrap()
                .exe_path,
            Some("bun".into())
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
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("bun")
                .unwrap()
                .exe_path,
            Some("bun.exe".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_globals_dirs() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;

        assert_eq!(
            output.globals_lookup_dirs,
            [
                "$BUN_INSTALL_BIN",
                "$BUN_INSTALL/bin",
                "$XDG_CACHE_HOME/.bun/bin",
                "$HOME/.bun/bin"
            ]
        );
    }
}
