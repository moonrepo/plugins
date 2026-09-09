use proto_pdk_test_utils::*;

mod python_tool {
    use super::*;

    generate_resolve_versions_tests!("python-test", {
        "2.3" => "2.3.7",
        "3.10.1" => "3.10.1",
        "3.10" => "3.10.21",
        // "3" => "3.12.4",
    });

    fn create_input(version: &str) -> ResolveVersionInput {
        ResolveVersionInput {
            initial: UnresolvedVersionSpec::parse(version).unwrap(),
            ..Default::default()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_prereleases_in_registry_format() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        let versions = output
            .versions
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>();

        assert!(versions.contains(&"3.14.0".to_string()));
        assert!(versions.contains(&"3.14.0-alpha.7".to_string()));
        assert!(versions.contains(&"3.14.0-beta.1".to_string()));
        assert!(versions.contains(&"3.14.0-rc.1".to_string()));
        assert!(!versions.contains(&"3.14.0-b.1".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn appends_latest_build_to_full_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        assert_eq!(
            plugin.resolve_version(create_input("3.12.0")).await,
            ResolveVersionOutput {
                version: Some(VersionSpec::parse("3.12.0+20231002").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn normalizes_short_prerelease_format() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        assert_eq!(
            plugin.resolve_version(create_input("3.14.0-b.1")).await,
            ResolveVersionOutput {
                candidate: Some(UnresolvedVersionSpec::parse("3.14.0-beta.1").unwrap()),
                version: Some(VersionSpec::parse("3.14.0-beta.1+20250604").unwrap()),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn normalizes_undotted_prerelease_format() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        let output = plugin.resolve_version(create_input("3.14.0-rc1")).await;

        assert_eq!(
            output.candidate,
            Some(UnresolvedVersionSpec::parse("3.14.0-rc.1").unwrap())
        );
        assert!(
            output
                .version
                .is_some_and(|v| v.to_string().starts_with("3.14.0-rc.1+"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_explicit_build() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        assert_eq!(
            plugin
                .resolve_version(create_input("3.12.0+20231002"))
                .await,
            ResolveVersionOutput::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_unknown_release() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        assert_eq!(
            plugin.resolve_version(create_input("3.0.0")).await,
            ResolveVersionOutput::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_ranges_and_aliases() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        for spec in ["3.12", "^3.12.0", "latest"] {
            assert_eq!(
                plugin.resolve_version(create_input(spec)).await,
                ResolveVersionOutput::default(),
                "for {spec}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }
}
