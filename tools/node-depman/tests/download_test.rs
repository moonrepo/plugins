use proto_pdk_test_utils::*;
use rustc_hash::FxHashMap;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct NodeDepmanPluginConfig {
    pub registry_url: String,
}

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
                        context: PluginContext {
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
                        context: PluginContext {
                            version: VersionSpec::parse("9.0.0").unwrap(),
                            ..Default::default()
                        },
                        install_dir: VirtualPath::Real(sandbox.path().into()),
                    })
                    .await
                    .exes
                    .get("npm")
                    .unwrap()
                    .exe_path,
                Some("shims/npm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn downloads_from_custom_registry() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("npm-test", |config| {
                    config
                        .host(HostOS::MacOS, HostArch::X64)
                        .tool_config(NodeDepmanPluginConfig {
                            registry_url: "https://some-internal-url.example".into(),
                        });
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("9.0.0").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url: "https://some-internal-url.example/npm/-/npm-9.0.0.tgz".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extracts_auth_token_header() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".npmrc", "//registry.npmjs.org/:_authToken = abc123");

            let plugin = sandbox
                .create_plugin_with_config("npm-test", |config| {
                    config.host(HostOS::Linux, HostArch::Arm64);
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("9.0.0").unwrap(),
                            working_dir: VirtualPath::Real(sandbox.path().into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url: "https://registry.npmjs.org/npm/-/npm-9.0.0.tgz".into(),
                    http_headers: FxHashMap::from_iter([(
                        "Authorization".into(),
                        "Bearer abc123".into()
                    )]),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extracts_auth_token_header_for_custom_registry() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".npmrc", "//registry.yarnpkg.com/:_authToken = abc123");

            let plugin = sandbox
                .create_plugin_with_config("npm-test", |config| {
                    config.host(HostOS::Linux, HostArch::Arm64).tool_config(
                        NodeDepmanPluginConfig {
                            registry_url: "https://registry.yarnpkg.com".into(),
                        },
                    );
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("9.0.0").unwrap(),
                            working_dir: VirtualPath::Real(sandbox.path().into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url: "https://registry.yarnpkg.com/npm/-/npm-9.0.0.tgz".into(),
                    http_headers: FxHashMap::from_iter([(
                        "Authorization".into(),
                        "Bearer abc123".into()
                    )]),
                    ..Default::default()
                }
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
                        context: PluginContext {
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
                        context: PluginContext {
                            version: VersionSpec::parse("8.0.0").unwrap(),
                            ..Default::default()
                        },
                        install_dir: VirtualPath::Real(sandbox.path().into()),
                    })
                    .await
                    .exes
                    .get("pnpm")
                    .unwrap()
                    .exe_path,
                Some("shims/pnpm.cmd".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn downloads_from_custom_registry() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("pnpm-test", |config| {
                    config
                        .host(HostOS::MacOS, HostArch::X64)
                        .tool_config(NodeDepmanPluginConfig {
                            registry_url: "https://some-internal-url.example".into(),
                        });
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("8.0.0").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url: "https://some-internal-url.example/pnpm/-/pnpm-8.0.0.tgz".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extracts_auth_token_header() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".npmrc", "//registry.npmjs.org/:_authToken = abc123");

            let plugin = sandbox
                .create_plugin_with_config("pnpm-test", |config| {
                    config.host(HostOS::Linux, HostArch::Arm64);
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("9.0.0").unwrap(),
                            working_dir: VirtualPath::Real(sandbox.path().into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url: "https://registry.npmjs.org/pnpm/-/pnpm-9.0.0.tgz".into(),
                    http_headers: FxHashMap::from_iter([(
                        "Authorization".into(),
                        "Bearer abc123".into()
                    )]),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extracts_auth_token_header_for_custom_registry() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".npmrc", "//registry.yarnpkg.com/:_authToken = abc123");

            let plugin = sandbox
                .create_plugin_with_config("pnpm-test", |config| {
                    config.host(HostOS::Linux, HostArch::Arm64).tool_config(
                        NodeDepmanPluginConfig {
                            registry_url: "https://registry.yarnpkg.com".into(),
                        },
                    );
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("9.0.0").unwrap(),
                            working_dir: VirtualPath::Real(sandbox.path().into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url: "https://registry.yarnpkg.com/pnpm/-/pnpm-9.0.0.tgz".into(),
                    http_headers: FxHashMap::from_iter([(
                        "Authorization".into(),
                        "Bearer abc123".into()
                    )]),
                    ..Default::default()
                }
            );
        }
    }

    mod yarn1 {
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
                        context: PluginContext {
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
                        context: PluginContext {
                            version: VersionSpec::parse("1.22.0").unwrap(),
                            ..Default::default()
                        },
                        install_dir: VirtualPath::Real(sandbox.path().into()),
                    })
                    .await
                    .exes
                    .get("yarn")
                    .unwrap()
                    .exe_path,
                Some("shims/yarn".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn downloads_from_custom_registry() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config
                        .host(HostOS::MacOS, HostArch::X64)
                        .tool_config(NodeDepmanPluginConfig {
                            registry_url: "https://registry.yarnpkg.com".into(),
                        });
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("1.22.0").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("yarn-v1.22.0".into()),
                    download_url: "https://registry.yarnpkg.com/yarn/-/yarn-1.22.0.tgz".into(),
                    ..Default::default()
                }
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
                        context: PluginContext {
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
                        context: PluginContext {
                            version: VersionSpec::parse("3.6.1").unwrap(),
                            ..Default::default()
                        },
                        install_dir: VirtualPath::Real(sandbox.path().into()),
                    })
                    .await
                    .exes
                    .get("yarn")
                    .unwrap()
                    .exe_path,
                Some("shims/yarn".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extracts_auth_token_header() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".yarnrc.yml", "npmAuthToken: abc123");

            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::Linux, HostArch::Arm64);
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("4.5.0").unwrap(),
                            working_dir: VirtualPath::Real(sandbox.path().into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url:
                        "https://registry.npmjs.org/@yarnpkg/cli-dist/-/cli-dist-4.5.0.tgz".into(),
                    http_headers: FxHashMap::from_iter([(
                        "Authorization".into(),
                        "Bearer abc123".into()
                    )]),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extracts_auth_token_header_for_custom_registry() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                ".yarnrc.yml",
                r#"
npmAuthToken: xyz789

npmRegistries:
    //registry.yarnpkg.com/:
        npmAuthToken: abc123
"#,
            );

            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::Linux, HostArch::Arm64).tool_config(
                        NodeDepmanPluginConfig {
                            registry_url: "https://registry.yarnpkg.com".into(),
                        },
                    );
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("4.5.0").unwrap(),
                            working_dir: VirtualPath::Real(sandbox.path().into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("package".into()),
                    download_url:
                        "https://registry.yarnpkg.com/@yarnpkg/cli-dist/-/cli-dist-4.5.0.tgz".into(),
                    http_headers: FxHashMap::from_iter([(
                        "Authorization".into(),
                        "Bearer abc123".into()
                    )]),
                    ..Default::default()
                }
            );
        }
    }

    mod yarn6 {
        use super::*;

        // Yarn >= 6 is Rust based and downloaded from GitHub releases,
        // not the npm registry: https://github.com/yarnpkg/zpm

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        mod install {
            use super::*;

            generate_download_install_tests!("yarn-test", "6.0.0-rc.19");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_prebuilt_macos_arm64() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::MacOS, HostArch::Arm64);
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("yarn-aarch64-apple-darwin".into()),
                    download_name: Some("yarn-aarch64-apple-darwin.zip".into()),
                    download_url: "https://github.com/yarnpkg/zpm/releases/download/v6.0.0-rc.19/yarn-aarch64-apple-darwin.zip".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_prebuilt_linux_arm64_musl() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
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
                            version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("yarn-aarch64-unknown-linux-musl".into()),
                    download_name: Some("yarn-aarch64-unknown-linux-musl.zip".into()),
                    download_url: "https://github.com/yarnpkg/zpm/releases/download/v6.0.0-rc.19/yarn-aarch64-unknown-linux-musl.zip".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_prebuilt_linux_x64_musl() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host_with(|host| {
                        host.os = HostOS::Linux;
                        host.arch = HostArch::X64;
                        host.libc = HostLibc::Musl;
                    });
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("yarn-x86_64-unknown-linux-musl".into()),
                    download_name: Some("yarn-x86_64-unknown-linux-musl.zip".into()),
                    download_url: "https://github.com/yarnpkg/zpm/releases/download/v6.0.0-rc.19/yarn-x86_64-unknown-linux-musl.zip".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Only musl is supported.")]
        async fn doesnt_support_linux_gnu() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host_with(|host| {
                        host.os = HostOS::Linux;
                        host.arch = HostArch::X64;
                        host.libc = HostLibc::Gnu;
                    });
                })
                .await;

            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "unsupported OS windows.")]
        async fn doesnt_support_windows() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::Windows, HostArch::X64);
                })
                .await;

            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_prebuilt_linux_x86_musl() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host_with(|host| {
                        host.os = HostOS::Linux;
                        host.arch = HostArch::X86;
                        host.libc = HostLibc::Musl;
                    });
                })
                .await;

            assert_eq!(
                plugin
                    .download_prebuilt(DownloadPrebuiltInput {
                        context: PluginContext {
                            version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .await,
                DownloadPrebuiltOutput {
                    archive_prefix: Some("yarn-i686-unknown-linux-musl".into()),
                    download_name: Some("yarn-i686-unknown-linux-musl.zip".into()),
                    download_url: "https://github.com/yarnpkg/zpm/releases/download/v6.0.0-rc.19/yarn-i686-unknown-linux-musl.zip".into(),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "unsupported architecture riscv64.")]
        async fn doesnt_support_other_archs() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::Linux, HostArch::Riscv64);
                })
                .await;

            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "yarn does not support canary/nightly versions.")]
        async fn doesnt_support_canary() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::MacOS, HostArch::Arm64);
                })
                .await;

            plugin
                .download_prebuilt(DownloadPrebuiltInput {
                    context: PluginContext {
                        version: VersionSpec::parse("canary").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_default_bin() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::MacOS, HostArch::Arm64);
                })
                .await;

            let exes = plugin
                .locate_executables(LocateExecutablesInput {
                    context: PluginContext {
                        version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                        ..Default::default()
                    },
                    install_dir: VirtualPath::Real(sandbox.path().into()),
                })
                .await
                .exes;

            assert_eq!(
                exes.get("yarn").unwrap().exe_path,
                Some("yarn-bin".into())
            );

            // The yarnpkg alias is not supported in v6
            assert!(!exes.contains_key("yarnpkg"));

            // No internal shims are created for the native binary
            assert!(!sandbox.path().join("shims/yarn").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_default_bin_windows() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("yarn-test", |config| {
                    config.host(HostOS::Windows, HostArch::X64);
                })
                .await;

            let exes = plugin
                .locate_executables(LocateExecutablesInput {
                    context: PluginContext {
                        version: VersionSpec::parse("6.0.0-rc.19").unwrap(),
                        ..Default::default()
                    },
                    install_dir: VirtualPath::Real(sandbox.path().into()),
                })
                .await
                .exes;

            // The .exe extension must not be rewritten to .cmd
            assert_eq!(
                exes.get("yarn").unwrap().exe_path,
                Some("yarn-bin.exe".into())
            );
        }
    }
}
