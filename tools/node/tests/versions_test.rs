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

mod node_tool {
    use super::*;
    use ::node_tool::NodeToolConfig;

    generate_resolve_versions_tests!("node-test", {
        "8" => "8.17.0",
        "10.1" => "10.1.0",
        "lts-gallium" => "16.20.2",
        "lts/fermium" => "14.21.3",
        // "stable" => "20.15.1",
        // "node" => "22.4.1",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_dist_url() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_custom_index_url() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.tool_config(NodeToolConfig {
                    index_url: "https://nodejs.org/download/nightly/index.json".into(),
                    ..Default::default()
                });
            })
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
        assert!(
            output
                .versions
                .iter()
                .all(|version| version.to_string().contains("nightly"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_lts_aliases() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        let mut aliases = output.aliases.keys().collect::<Vec<_>>();
        aliases.sort();

        assert_eq!(
            aliases,
            [
                "argon", "boron", "carbon", "dubnium", "erbium", "fermium", "gallium", "hydrogen",
                "iron", "jod", "krypton", "latest", "lithium", "stable"
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_engines() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "engines": { "node": ">=16" } }"#
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=16").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_dev_engines() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "devEngines": { "runtime": { "name": "node", "version": ">=16" } } }"#
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=16").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_volta() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "volta": { "node": "16.20.2" } }"#
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("16.20.2").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_first_field_when_multiple_are_valid() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "devEngines": { "runtime": { "name": "node", "version": "^20" } }, "volta": { "node": "18.1.0" }, "engines": { "node": ">=16" } }"#,
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("^20").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_invalid_field_and_parses_next_valid_field() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "package.json",
                    r#"{ "volta": { "node": "invalid version" }, "engines": { "node": ">=18" } }"#
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=18").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "Failed to parse a version requirement.")]
    async fn errors_if_no_field_is_valid() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        plugin
            .parse_version_file(version_file(
                &sandbox,
                "package.json",
                r#"{ "volta": { "node": "invalid version" } }"#,
            ))
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_nvmrc() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(&sandbox, ".nvmrc", "~20"))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("~20").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_nvmrc_with_comment() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(&sandbox, ".nvmrc", "# comment\n^20.1 # LTS"))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("^20.1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_node_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(&sandbox, ".node-version", "~20"))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("~20").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_node_version_with_comment() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    ".node-version",
                    "# comment\n^20.1 # LTS"
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("^20.1").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_latest_lts_wildcard() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        for (file, content) in [
            (".nvmrc", "lts/*"),
            (".nvmrc", "lts-*\n"),
            (".node-version", "lts/*"),
        ] {
            assert_eq!(
                plugin
                    .parse_version_file(version_file(&sandbox, file, content))
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::Alias("lts".into())),
                }
            );
        }
    }

    mod pin_version {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_for_aliases() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("package.json", "{}");

            let plugin = sandbox.create_plugin("node-test").await;

            let output = plugin
                .pin_version(PinVersionInput {
                    dir: plugin.tool.to_virtual_path(sandbox.path()),
                    version: UnresolvedVersionSpec::parse("lts").unwrap(),
                    ..Default::default()
                })
                .await;

            assert!(!output.pinned);
            assert!(output.error.unwrap().contains("not aliases"));
            assert_eq!(
                std::fs::read_to_string(sandbox.path().join("package.json")).unwrap(),
                "{}"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_if_no_package_json() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("node-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse(">=20").unwrap(),
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

            let plugin = sandbox.create_plugin("node-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse(">=20").unwrap(),
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
      "name": "node",
      "version": "^18"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("node-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse(">=20").unwrap(),
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
      "name": "bun",
      "version": "^1"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("node-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: plugin.tool.to_virtual_path(sandbox.path()),
                        version: UnresolvedVersionSpec::parse(">=20").unwrap(),
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
        async fn errors_if_no_package_json() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("node-test").await;

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
      "name": "node",
      "version": ">=20"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("node-test").await;

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
                    version: UnresolvedVersionSpec::parse(">=20").ok(),
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

            let plugin = sandbox.create_plugin("node-test").await;

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
                    version: UnresolvedVersionSpec::parse(">=20").ok(),
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
      "name": "bun",
      "version": "^1"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("node-test").await;

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
