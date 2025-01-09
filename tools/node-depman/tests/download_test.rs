use proto_pdk_test_utils::*;

mod node_depman_tool {
    use super::*;

    mod npm {
        use super::*;

        generate_download_install_tests!("npm-test", "9.0.0");

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_prebuilt() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("npm-test", |config| {
                    config.host(HostOS::Linux, HostArch::Arm64);
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: ToolContext {
                            version: VersionSpec::parse("9.0.0").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url: "https://registry.npmjs.org/npm/-/npm-9.0.0.tgz".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_default_bin() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("npm-test", |config| {
                    config.host(HostOS::Linux, HostArch::Arm64);
                })
                .await;

            assert_eq!(
                plugin
                    .locate_executables(LocateExecutablesInput {
                        context: ToolContext {
                            version: VersionSpec::parse("9.0.0").unwrap(),
                            ..Default::default()
                        },
                    })
                    .await
                    .exes
                    .get("npm")
                    .unwrap()
                    .exe_path,
                Some("bin/npm-cli.js".into())
            );
        }
    }

    mod pnpm {
        use super::*;

        generate_download_install_tests!("pnpm-test", "8.0.0");

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_prebuilt() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("pnpm-test", |config| {
                    config.host(HostOS::Windows, HostArch::X64);
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: ToolContext {
                            version: VersionSpec::parse("8.0.0").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url: "https://registry.npmjs.org/pnpm/-/pnpm-8.0.0.tgz".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_default_bin() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("pnpm-test", |config| {
                    config.host(HostOS::Windows, HostArch::X64);
                })
                .await;

            assert_eq!(
                plugin
                    .locate_executables(LocateExecutablesInput {
                        context: ToolContext {
                            version: VersionSpec::parse("8.0.0").unwrap(),
                            ..Default::default()
                        },
                    })
                    .await
                    .exes
                    .get("pnpm")
                    .unwrap()
                    .exe_path,
                Some("bin/pnpm.cjs".into())
            );
        }
    }

    mod yarn {
        use super::*;

        generate_download_install_tests!("yarn-test", "1.22.0");

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_prebuilt() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::MacOS, HostArch::X64);
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: ToolContext {
                            version: VersionSpec::parse("1.22.0").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("yarn-v1.22.0".into()),
                    download_url: "https://registry.npmjs.org/yarn/-/yarn-1.22.0.tgz".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_default_bin() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::MacOS, HostArch::X64);
                })
                .await;

            assert_eq!(
                plugin
                    .locate_executables(LocateExecutablesInput {
                        context: ToolContext {
                            version: VersionSpec::parse("1.22.0").unwrap(),
                            ..Default::default()
                        },
                    })
                    .await
                    .exes
                    .get("yarn")
                    .unwrap()
                    .exe_path,
                Some("bin/yarn.js".into())
            );
        }
    }

    mod yarn2 {
        use super::*;

        // Special case
        generate_download_install_tests!("yarn-test", "2.4.3");
    }

    mod yarn_berry {
        use super::*;

        generate_download_install_tests!("yarn-test", "3.6.1");

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_prebuilt() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::MacOS, HostArch::X64);
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: ToolContext {
                            version: VersionSpec::parse("3.6.1").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url:
                        "https://registry.npmjs.org/@yarnpkg/cli-dist/-/cli-dist-3.6.1.tgz".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_default_bin() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::MacOS, HostArch::X64);
                })
                .await;

            assert_eq!(
                plugin
                    .locate_executables(LocateExecutablesInput {
                        context: ToolContext {
                            version: VersionSpec::parse("3.6.1").unwrap(),
                            ..Default::default()
                        },
                    })
                    .await
                    .exes
                    .get("yarn")
                    .unwrap()
                    .exe_path,
                Some("bin/yarn.js".into())
            );
        }
    }
}
