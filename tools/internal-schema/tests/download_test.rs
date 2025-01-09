#![allow(clippy::disallowed_names)]

use proto_pdk_test_utils::*;
use starbase_sandbox::locate_fixture;
use std::collections::HashMap;

mod schema_tool {
    use super::*;

    generate_download_install_tests!(
        "schema-test",
        "1.10.0",
        Some(locate_fixture("schemas").join("base.toml"))
    );

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::Linux, HostArch::Arm64);
                },
            )
            .await;

        assert_eq!(
        plugin.download_prebuilt(DownloadPrebuiltInput {
            context: ToolContext {
                version: VersionSpec::parse("20.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await,
        DownloadPrebuiltOutput {
            archive_prefix: Some("moon-linux-aarch64-20.0.0".into()),
            checksum_name: Some("CHECKSUM.txt".into()),
            download_name: Some("moon-aarch64-unknown-linux-gnu".into()),
            download_url: "https://github.com/moonrepo/moon/releases/download/v20.0.0/moon-aarch64-unknown-linux-gnu".into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                },
            )
            .await;

        assert_eq!(
        plugin.download_prebuilt(DownloadPrebuiltInput {
            context: ToolContext {
                version: VersionSpec::parse("20.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await,
        DownloadPrebuiltOutput {
            archive_prefix: Some("moon-linux-x86_64-20.0.0".into()),
            checksum_name: Some("CHECKSUM.txt".into()),
            download_name: Some("moon-x86_64-unknown-linux-gnu".into()),
            download_url: "https://github.com/moonrepo/moon/releases/download/v20.0.0/moon-x86_64-unknown-linux-gnu".into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::MacOS, HostArch::Arm64);
                },
            )
            .await;

        assert_eq!(
        plugin.download_prebuilt(DownloadPrebuiltInput {
            context: ToolContext {
                version: VersionSpec::parse("20.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await,
        DownloadPrebuiltOutput {
            archive_prefix: None,
            checksum_name: Some("SHASUM256.txt".into()),
            download_name: Some("moon-aarch64-apple-darwin".into()),
            download_url: "https://github.com/moonrepo/moon/releases/download/v20.0.0/moon-aarch64-apple-darwin".into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::MacOS, HostArch::X64);
                },
            )
            .await;

        assert_eq!(
        plugin.download_prebuilt(DownloadPrebuiltInput {
            context: ToolContext {
                version: VersionSpec::parse("20.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await,
        DownloadPrebuiltOutput {
            archive_prefix: None,
            checksum_name: Some("SHASUM256.txt".into()),
            download_name: Some("moon-x86_64-apple-darwin".into()),
            download_url: "https://github.com/moonrepo/moon/releases/download/v20.0.0/moon-x86_64-apple-darwin".into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::Windows, HostArch::Arm64);
                },
            )
            .await;

        assert_eq!(
        plugin.download_prebuilt(DownloadPrebuiltInput {
            context: ToolContext {
                version: VersionSpec::parse("20.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await,
        DownloadPrebuiltOutput {
            archive_prefix: None,
            checksum_name: Some("CHECKSUM.txt".into()),
            download_name: Some("moon-aarch64-pc-windows-msvc.exe".into()),
            download_url: "https://github.com/moonrepo/moon/releases/download/v20.0.0/moon-aarch64-pc-windows-msvc.exe".into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x86() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::Windows, HostArch::X86);
                },
            )
            .await;

        assert_eq!(
        plugin.download_prebuilt(DownloadPrebuiltInput {
            context: ToolContext {
                version: VersionSpec::parse("20.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await,
        DownloadPrebuiltOutput {
            archive_prefix: None,
            checksum_name: Some("CHECKSUM.txt".into()),
            download_name: Some("moon-x86-pc-windows-msvc.exe".into()),
            download_url: "https://github.com/moonrepo/moon/releases/download/v20.0.0/moon-x86-pc-windows-msvc.exe".into(),
            ..Default::default()
        }
    );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_linux_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::Linux, HostArch::Arm64);
                },
            )
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                })
                .await
                .exes
                .get("schema-test")
                .unwrap()
                .exe_path,
            Some("lin/moon".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_macos_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::MacOS, HostArch::X64);
                },
            )
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                })
                .await
                .exes
                .get("schema-test")
                .unwrap()
                .exe_path,
            Some("mac/moon".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin_with_config(
                "schema-test",
                locate_fixture("schemas").join("bins.toml"),
                |config| {
                    config.host(HostOS::Windows, HostArch::X64);
                },
            )
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                })
                .await
                .exes
                .get("schema-test")
                .unwrap()
                .exe_path,
            Some("win/moon.exe".into())
        );
    }

    mod primary {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_primary_config() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_schema_plugin_with_config(
                    "schema-test",
                    locate_fixture("schemas").join("primary.toml"),
                    |config| {
                        config.host(HostOS::MacOS, HostArch::X64);
                    },
                )
                .await;

            let result = plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                })
                .await;

            let config = result.exes.get("schema-test").unwrap();

            assert_eq!(config.exe_path, Some("bin/moon".into()));
            assert!(config.no_shim);
            assert_eq!(
                config.shim_before_args,
                Some(StringOrVec::Vec(vec!["-v".into()]))
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn auto_adds_exe_to_bin_on_windows() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_schema_plugin_with_config(
                    "schema-test",
                    locate_fixture("schemas").join("primary.toml"),
                    |config| {
                        config.host(HostOS::Windows, HostArch::X64);
                    },
                )
                .await;

            assert_eq!(
                plugin
                    .locate_executables(LocateExecutablesInput {
                        context: ToolContext {
                            version: VersionSpec::parse("20.0.0").unwrap(),
                            ..Default::default()
                        },
                    })
                    .await
                    .exes
                    .get("schema-test")
                    .unwrap()
                    .exe_path,
                Some("bin/moon.exe".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn primary_path_doesnt_override_platform_path() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_schema_plugin_with_config(
                    "schema-test",
                    locate_fixture("schemas").join("primary-platform.toml"),
                    |config| {
                        config.host(HostOS::Linux, HostArch::X64);
                    },
                )
                .await;

            assert_eq!(
                plugin
                    .locate_executables(LocateExecutablesInput {
                        context: ToolContext {
                            version: VersionSpec::parse("20.0.0").unwrap(),
                            ..Default::default()
                        },
                    })
                    .await
                    .exes
                    .get("schema-test")
                    .unwrap()
                    .exe_path,
                Some("lin/moon".into())
            );
        }
    }

    mod secondary {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_secondary_config() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_schema_plugin_with_config(
                    "schema-test",
                    locate_fixture("schemas").join("secondary.toml"),
                    |config| {
                        config.host(HostOS::MacOS, HostArch::X64);
                    },
                )
                .await;

            let result = plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                })
                .await;

            let foo = result.exes.get("foo").unwrap();

            assert_eq!(foo.exe_path, Some("bin/foo".into()));

            let bar = result.exes.get("bar").unwrap();

            assert_eq!(bar.exe_path, Some("bin/bar".into()));
            assert!(bar.no_bin);
            assert_eq!(
                bar.shim_env_vars,
                Some(HashMap::from_iter([("BAR".into(), "bar".into())]))
            );

            let baz = result.exes.get("baz").unwrap();

            assert_eq!(baz.exe_path, Some("bin/baz".into()));
            assert_eq!(baz.exe_link_path, Some("bin/baz-link".into()));
            assert!(baz.no_shim);

            let qux = result.exes.get("qux").unwrap();

            assert_eq!(qux.exe_path, Some("bin/qux.js".into()));
            assert_eq!(qux.parent_exe_name, Some("node".into()));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn auto_adds_exe_to_bin_on_windows() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_schema_plugin_with_config(
                    "schema-test",
                    locate_fixture("schemas").join("secondary.toml"),
                    |config| {
                        config.host(HostOS::Windows, HostArch::X64);
                    },
                )
                .await;

            let result = plugin
                .locate_executables(LocateExecutablesInput {
                    context: ToolContext {
                        version: VersionSpec::parse("20.0.0").unwrap(),
                        ..Default::default()
                    },
                })
                .await;

            assert_eq!(
                result.exes.get("foo").unwrap().exe_path,
                Some("bin/foo.exe".into())
            );
            assert_eq!(
                result.exes.get("bar").unwrap().exe_path,
                Some("bin/bar.exe".into())
            );
            assert_eq!(
                result.exes.get("baz").unwrap().exe_path,
                Some("bin/baz.exe".into())
            );
            assert_eq!(
                result.exes.get("qux").unwrap().exe_path,
                Some("bin/qux.js".into())
            );
        }
    }
}
