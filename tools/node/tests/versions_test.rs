use proto_pdk_test_utils::*;
use starbase_sandbox::assert_snapshot;
use std::fs;

mod node_tool {
    use super::*;

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
                "iron", "jod", "krypton", "latest", "stable"
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_engines() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: r#"{ "engines": { "node": ">=16" } }"#.into(),
                    file: "package.json".into(),
                    ..Default::default()
                })
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
                .parse_version_file(ParseVersionFileInput {
                    content:
                        r#"{ "devEngines": { "runtime": { "name": "node", "version": ">=16" } } }"#
                            .into(),
                    file: "package.json".into(),
                    ..Default::default()
                })
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
                .parse_version_file(ParseVersionFileInput {
                    content: r#"{ "volta": { "node": "16.20.2" } }"#.into(),
                    file: "package.json".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("16.20.2").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_nvmrc() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("node-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: "~20".into(),
                    file: ".nvmrc".into(),
                    ..Default::default()
                })
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
                .parse_version_file(ParseVersionFileInput {
                    content: "# comment\n^20.1".into(),
                    file: ".nvmrc".into(),
                    ..Default::default()
                })
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
                .parse_version_file(ParseVersionFileInput {
                    content: "~20".into(),
                    file: ".node-version".into(),
                    ..Default::default()
                })
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
                .parse_version_file(ParseVersionFileInput {
                    content: "# comment\n^20.1".into(),
                    file: ".node-version".into(),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("^20.1").unwrap()),
            }
        );
    }

    mod pin_version {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_if_no_package_json() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("node-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: VirtualPath::Real(sandbox.path().into()),
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
                        dir: VirtualPath::Real(sandbox.path().into()),
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
                        dir: VirtualPath::Real(sandbox.path().into()),
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
                        dir: VirtualPath::Real(sandbox.path().into()),
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
                        dir: VirtualPath::Real(sandbox.path().into()),
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
                        dir: VirtualPath::Real(sandbox.path().into()),
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
                        dir: VirtualPath::Real(sandbox.path().into()),
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
                        dir: VirtualPath::Real(sandbox.path().into()),
                        ..Default::default()
                    })
                    .await,
                UnpinVersionOutput::default(),
            );

            assert_snapshot!(fs::read_to_string(sandbox.path().join("package.json")).unwrap());
        }
    }
}
