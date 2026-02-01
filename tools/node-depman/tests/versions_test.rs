use proto_pdk_test_utils::*;
use starbase_sandbox::assert_snapshot;
use std::fs;

mod node_depman_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_volta_extends() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("package.json", r#"{ "volta": { "extends": "./a.json" } }"#);
        sandbox.create_file("a.json", r#"{ "volta": { "extends": "./b.json" } }"#);
        sandbox.create_file("b.json", r#"{ "volta": { "npm": "1.2.3" } }"#);

        let plugin = sandbox.create_plugin("npm-test").await;

        assert_eq!(
            plugin
                .parse_version_file(ParseVersionFileInput {
                    content: r#"{ "volta": { "extends": "./a.json" } }"#.into(),
                    file: "package.json".into(),
                    path: VirtualPath::Real(sandbox.path().join("package.json")),
                    ..Default::default()
                })
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
            }
        );
    }

    mod npm {
        use super::*;

        generate_resolve_versions_tests!("npm-test", {
            "7" => "7.24.2",
            "8.1" => "8.1.4",
            "9.7.2" => "9.7.2",
        });

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_parse_package_manager_if_diff_name() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "yarn@1.2.3" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput { version: None }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package_manager() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "npm@1.2.3" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package_manager_with_hash() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
            plugin.parse_version_file(ParseVersionFileInput {
                content: r#"{ "packageManager": "npm@1.2.3+sha256.c362077587b1e782e5aef3dcf85826399ae552ad66b760e2585c4ac11102243f" }"#.into(),
                file: "package.json".into(),
                    ..Default::default()
            }).await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
            }
        );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package_manager_latest() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "npm" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("latest").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_engines() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "engines": { "npm": "1.2.3" } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_dev_engines() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "devEngines": { "packageManager": { "name": "npm", "version": "1.2.3" } } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_volta() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "volta": { "npm": "1.2.3" } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }
    }

    mod pnpm {
        use super::*;

        generate_resolve_versions_tests!("pnpm-test", {
            "7" => "7.33.7",
            "8.1" => "8.1.1",
            "dev" => "6.23.7-202112041634",
        });

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_parse_package_manager_if_diff_name() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("pnpm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "yarn@1.2.3" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput { version: None }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package_manager() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("pnpm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "pnpm@1.2.3" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package_manager_latest() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("pnpm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "pnpm" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("latest").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_engines() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("pnpm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "engines": { "pnpm": "1.2.3" } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_dev_engines() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.enable_logging();
            let plugin = sandbox.create_plugin("pnpm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "devEngines": { "packageManager": { "name": "pnpm", "version": "1.2.3" } } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_volta() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("pnpm-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "volta": { "pnpm": "1.2.3" } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }
    }

    mod yarn {
        use super::*;

        generate_resolve_versions_tests!("yarn-test", {
            "1" => "1.22.22",
            "2" => "2.4.3",
            "3" => "3.8.7",
            // "berry" => "4.3.1",
        });

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_parse_package_manager_if_diff_name() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("yarn-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "pnpm@1.2.3" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput { version: None }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package_manager() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("yarn-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "yarn@1.2.3" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_package_manager_latest() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("yarn-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "packageManager": "yarn" }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("latest").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_engines() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("yarn-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "engines": { "yarn": "1.2.3" } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_dev_engines() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("yarn-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "devEngines": { "packageManager": { "name": "yarn", "version": "1.2.3" } } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn parses_volta() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("yarn-test").await;

            assert_eq!(
                plugin
                    .parse_version_file(ParseVersionFileInput {
                        content: r#"{ "volta": { "yarn": "1.2.3" } }"#.into(),
                        file: "package.json".into(),
                        ..Default::default()
                    })
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse("1.2.3").unwrap()),
                }
            );
        }
    }

    mod pin_version {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn errors_if_no_package_json() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: VirtualPath::Real(sandbox.path().into()),
                        version: UnresolvedVersionSpec::parse(">=10").unwrap(),
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

            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: VirtualPath::Real(sandbox.path().into()),
                        version: UnresolvedVersionSpec::parse(">=10").unwrap(),
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
    "packageManager": {
      "name": "npm",
      "version": "^9"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: VirtualPath::Real(sandbox.path().into()),
                        version: UnresolvedVersionSpec::parse(">=10").unwrap(),
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
    "packageManager": {
      "name": "pnpm",
      "version": "^10"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("npm-test").await;

            assert_eq!(
                plugin
                    .pin_version(PinVersionInput {
                        dir: VirtualPath::Real(sandbox.path().into()),
                        version: UnresolvedVersionSpec::parse(">=10").unwrap(),
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
            let plugin = sandbox.create_plugin("yarn-test").await;

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
    "packageManager": {
      "name": "yarn",
      "version": ">=2"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("yarn-test").await;

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
                    version: UnresolvedVersionSpec::parse(">=2").ok(),
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
    "packageManager": [
      {
        "name": "yarn",
        "version": ">=2"
      },
      {
        "name": "pnpm",
        "version": "^10"
      }
    ]
  }
}"#,
            );

            let plugin = sandbox.create_plugin("yarn-test").await;

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
                    version: UnresolvedVersionSpec::parse(">=2").ok(),
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
    "packageManager": {
      "name": "pnpm",
      "version": "^10"
    }
  }
}"#,
            );

            let plugin = sandbox.create_plugin("yarn-test").await;

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
