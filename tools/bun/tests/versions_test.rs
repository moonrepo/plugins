use proto_pdk_test_utils::*;
use starbase_sandbox::assert_snapshot;
use std::fs;

fn version_file(
    sandbox: &ProtoWasmSandbox,
    file: &str,
    content: impl AsRef<str>,
) -> ParseVersionFileInput {
    sandbox.create_file(file, content);

    ParseVersionFileInput {
        file: file.into(),
        path: VirtualPath::new(sandbox.path().join(file)),
        ..Default::default()
    }
}

mod bun_tool {
    use super::*;

    generate_resolve_versions_tests!("bun-test", {
        "0.4" => "0.4.0",
        "0.5.1" => "0.5.1",
        "1.1.0" => "1.1.0",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_engines() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "engines": { "bun": ">=1" } }"#
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn prefers_package_manager_over_engines() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "engines": { "bun": ">=1" }, "packageManager": "bun@1.2.3" }"#
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_dev_engines() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "devEngines": { "runtime": { "name": "bun", "version": ">=1" } } }"#
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_volta() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "volta": { "bun": "1.20.2" } }"#
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.20.2").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_bumrc() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(&sandbox, ".bumrc", "~2"))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("~2").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_bumrc_with_comment() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(&sandbox, ".bumrc", "# comment\n^2.1 # stable"))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("^2.1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_bun_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(&sandbox, ".bun-version", "~2"))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("~2").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_bun_version_with_comment() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("bun-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    ".bun-version",
                    "# comment\n^2.1 # stable"
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("^2.1").unwrap()),
            }
        );
    }

    mod pin_version {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn updates_existing_version_file() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".bun-version", "1.1.0\n");
            sandbox.create_file("package.json", "{}");

            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse("1.2.3").unwrap(),
                        ..Default::default()
                    })
                    .await,
                PinVersionOutput {
                    file: Some(
                        plugin
                            .tool
                            .to_virtual_path(sandbox.path().join(".bun-version"))
                    ),
                    error: None,
                    pinned: true,
                }
            );

            assert_eq!(
                fs::read_to_string(sandbox.path().join(".bun-version")).unwrap(),
                "1.2.3\n"
            );
            assert_eq!(
                fs::read_to_string(sandbox.path().join("package.json")).unwrap(),
                "{}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_for_aliases_in_package_json() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("package.json", "{}");

            let plugin = sandbox.create_plugin("bun-test").await;

            let output = plugin
                .pin_version(PinVersionInput {
                    dir: plugin.tool.to_virtual_path(sandbox.path()),
                    version: UnresolvedVersionSpec::parse("latest").unwrap(),
                    ..Default::default()
                })
                .await;

            assert!(!output.pinned);
            assert!(output.error.unwrap().contains("not aliases"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_if_no_package_json() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse(">=1").unwrap(),
                        ..Default::default()
                    })
                    .await,
                PinVersionOutput {
                    file: None,
                    error: Some(
                        "No <file>package.json</file> exists in the target directory.".into()
                    ),
                    pinned: false,
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inserts_when_missing() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("package.json", "{}");

            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse(">=1").unwrap(),
                        ..Default::default()
                    })
                    .await,
                PinVersionOutput {
                    file: Some(
                        plugin
                            .tool
                            .to_virtual_path(sandbox.path().join("package.json"))
                    ),
                    error: None,
                    pinned: true,
                }
            );

            assert_snapshot!(fs::read_to_string(sandbox.path().join("package.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn updates_when_matching() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{
  "devEngines": {
    "runtime": {
      "name": "bun",
      "version": "^0"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse(">=1").unwrap(),
                        ..Default::default()
                    })
                    .await,
                PinVersionOutput {
                    file: Some(
                        plugin
                            .tool
                            .to_virtual_path(sandbox.path().join("package.json"))
                    ),
                    error: None,
                    pinned: true,
                }
            );

            assert_snapshot!(fs::read_to_string(sandbox.path().join("package.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_to_list_when_not_matching() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "^20"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse(">=1").unwrap(),
                        ..Default::default()
                    })
                    .await,
                PinVersionOutput {
                    file: Some(
                        plugin
                            .tool
                            .to_virtual_path(sandbox.path().join("package.json"))
                    ),
                    error: None,
                    pinned: true,
                }
            );

            assert_snapshot!(fs::read_to_string(sandbox.path().join("package.json")).unwrap());
        }
    }

    mod unpin_version {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_version_file() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(".bun-version", "1.2.3\n");

            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .unpin_version(UnpinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    })
                    .await,
                UnpinVersionOutput {
                    file: Some(
                        plugin
                            .tool
                            .to_virtual_path(sandbox.path().join(".bun-version"))
                    ),
                    unpinned: true,
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                    ..Default::default()
                }
            );

            assert!(!sandbox.path().join(".bun-version").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_if_no_package_json() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .unpin_version(UnpinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    })
                    .await,
                UnpinVersionOutput {
                    error: Some(
                        "No <file>package.json</file> exists in the target directory.".into()
                    ),
                    ..Default::default()
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_if_matching() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{
  "devEngines": {
    "runtime": {
      "name": "bun",
      "version": ">=0"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .unpin_version(UnpinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    })
                    .await,
                UnpinVersionOutput {
                    file: Some(
                        plugin
                            .tool
                            .to_virtual_path(sandbox.path().join("package.json"))
                    ),
                    error: None,
                    unpinned: true,
                    version: UnresolvedVersionSpec::parse(">=0").ok(),
                }
            );

            assert_snapshot!(fs::read_to_string(sandbox.path().join("package.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_from_list_if_matching() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{
  "devEngines": {
    "runtime": [
      {
        "name": "node",
        "version": ">=20"
      },
      {
        "name": "bun",
        "version": "^1"
      }
    ]
  }
}"#,
            );

            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .unpin_version(UnpinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    })
                    .await,
                UnpinVersionOutput {
                    file: Some(
                        plugin
                            .tool
                            .to_virtual_path(sandbox.path().join("package.json"))
                    ),
                    error: None,
                    unpinned: true,
                    version: UnresolvedVersionSpec::parse("^1").ok(),
                }
            );

            assert_snapshot!(fs::read_to_string(sandbox.path().join("package.json")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_remove_if_not_matching() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "^20"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("bun-test").await;

            assert_eq!(
                plugin
                    .unpin_version(UnpinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        ..Default::default()
                    })
                    .await,
                UnpinVersionOutput::default(),
            );

            assert_snapshot!(fs::read_to_string(sandbox.path().join("package.json")).unwrap());
        }
    }
}
