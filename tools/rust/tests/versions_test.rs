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

mod rust_tool {
    use super::*;

    generate_resolve_versions_tests!("rust-test", {
        "stable" => "stable",
        "nightly" => "nightly",
        // "nightly-2023-07-03" => "nightly-2023-07-03",
        "1.60" => "1.60.0",
        "1.71.1" => "1.71.1",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git_tags() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_rust_toolchain() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "rust-toolchain",
                    "1.60.0 # pinned for CI\n"
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.60.0").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn detects_legacy_file_first() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .detect_version_files(DetectVersionInput::default())
                .await
                .files,
            ["rust-toolchain", "rust-toolchain.toml"]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_rust_toolchain_as_toml() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "rust-toolchain",
                    "[toolchain]\nchannel = \"1.70.0\"\ncomponents = [\"clippy\"]"
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.70.0").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ignores_empty_rust_toolchain() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(&sandbox, "rust-toolchain", ""))
                .await,
            ParseVersionFileOutput::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_rust_toolchain_toml() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "rust-toolchain.toml",
                    "[toolchain]\nchannel = \"1.70.0\""
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.70.0").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_partial_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        // Logs a warning, but still resolves
        assert_eq!(
            plugin
                .parse_version_file(version_file(
                    &sandbox,
                    "rust-toolchain.toml",
                    "[toolchain]\nchannel = \"1.70\""
                ))
                .await,
            ParseVersionFileOutput {
                version: Some(UnresolvedVersionSpec::parse("1.70").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn ignores_empty_rust_toolchain_toml() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("rust-test").await;

        assert_eq!(
            plugin
                .parse_version_file(version_file(&sandbox, "rust-toolchain.toml", "[toolchain]"))
                .await,
            ParseVersionFileOutput::default()
        );
    }
}
