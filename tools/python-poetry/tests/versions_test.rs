use proto_pdk_test_utils::*;

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

mod python_poetry_tool {
    use super::*;

    generate_resolve_versions_tests!("poetry-test", {
        "1.8" => "1.8.5",
        "2.1.1" => "2.1.1",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("poetry-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("poetry-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_poetry_version_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("poetry-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    ".poetry-version",
                    "2.1.1 # pinned for CI\n"
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("2.1.1").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_pyproject_toml() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("poetry-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "pyproject.toml",
                    "[tool.poetry]\nrequires-poetry = \">=2.0\""
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse(">=2.0.0").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ignores_untranslatable_pyproject_toml_specifiers() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("poetry-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "pyproject.toml",
                    "[tool.poetry]\nrequires-poetry = \"~=2\""
                ))
                .await,
            ParseVersionFileOutput::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_pyproject_toml_pep_specifiers() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("poetry-test").await;

        for (constraint, expected) in [
            ("~=2.1", ">=2.1.0, =2.*"),
            ("~= 2.1.3", ">=2.1.3, =2.1.*"),
            (">=2.0, ~=2.1", ">=2.0.0, >=2.1.0, =2.*"),
            (">=2.0,<3.0", ">=2.0.0, <3.0.0"),
            ("==2.1.*", "=2.1.*"),
            ("==2.1", "=2.1.0"),
            (">=2.0, !=2.1.0", ">=2.0.0, !=2.1.0"),
        ] {
            assert_eq!(
                plugin
                    .parse_version_file(version_file(
                        &sandbox,
                        "pyproject.toml",
                        format!("[tool.poetry]\nrequires-poetry = \"{constraint}\"")
                    ))
                    .await,
                ParseVersionFileOutput {
                    version: Some(UnresolvedVersionSpec::parse(expected).unwrap()),
                    ..Default::default()
                },
                "{constraint}"
            );
        }
    }
}
