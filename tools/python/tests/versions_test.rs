use proto_pdk_test_utils::*;

mod python_tool {
    use super::*;
    use ::python_tool::PythonToolConfig;

    fn create_input(version: &str) -> ResolveVersionInput {
        ResolveVersionInput {
            initial: UnresolvedVersionSpec::parse(version).unwrap(),
            ..Default::default()
        }
    }

    async fn resolve(version: &str) -> ResolveVersionOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        plugin.resolve_version(create_input(version)).await
    }

    async fn resolve_with_latest_build(version: &str) -> ResolveVersionOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.tool_config(PythonToolConfig {
                    use_latest_build: true,
                });
            })
            .await;

        plugin.resolve_version(create_input(version)).await
    }

    generate_resolve_versions_tests!("python-test", {
        "2.3" => "2.3.7",
        "3.10.1" => "3.10.1",
        // "3" => "3.12.4",
    });

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

    // Build specific versions are part of the version list now,
    // so ranges resolve to the latest build instead of the plain version
    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_range_to_build_specific_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;
        let mut spec = ToolSpec::parse("3.10").unwrap();

        flow::resolve::Resolver::new(&plugin.tool)
            .resolve_version(&mut spec, false)
            .await
            .unwrap();

        let resolved = spec.get_resolved_version().to_string();

        assert!(resolved.starts_with("3.10."), "got {resolved}");
        assert!(resolved.contains('+'), "got {resolved}");
    }

    // These are our own builds, so they only exist in the registry
    #[tokio::test(flavor = "multi_thread")]
    async fn includes_build_specific_versions() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("python-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        let versions = output
            .versions
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>();

        assert!(versions.contains(&"3.12.0+20231002".to_string()));
        assert!(versions.contains(&"3.10.0+20211017".to_string()));
        assert!(versions.contains(&"3.10.0+20211012".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_append_build_by_default() {
        assert_eq!(resolve("3.12.0").await, ResolveVersionOutput::default());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn appends_latest_build_when_enabled() {
        assert_eq!(
            resolve_with_latest_build("3.12.0").await,
            ResolveVersionOutput {
                version: Some(VersionSpec::parse("3.12.0+20231002").unwrap()),
                ..Default::default()
            }
        );
    }

    // Has builds 20211017 and 20211012, the latest being first
    #[tokio::test(flavor = "multi_thread")]
    async fn appends_latest_of_many_builds_when_enabled() {
        assert_eq!(
            resolve_with_latest_build("3.10.0").await,
            ResolveVersionOutput {
                version: Some(VersionSpec::parse("3.10.0+20211017").unwrap()),
                ..Default::default()
            }
        );
    }

    // An explicit build always wins over the setting
    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_explicit_build_when_enabled() {
        assert_eq!(
            resolve_with_latest_build("3.10.0+20211012").await,
            ResolveVersionOutput::default()
        );
    }

    // Ranges and aliases are resolved from the version list instead
    #[tokio::test(flavor = "multi_thread")]
    async fn skips_ranges_and_aliases_when_enabled() {
        for spec in ["3.12", "^3.12.0", "latest"] {
            assert_eq!(
                resolve_with_latest_build(spec).await,
                ResolveVersionOutput::default(),
                "for {spec}"
            );
        }
    }

    // Normalizing the prerelease is not gated by the setting
    #[tokio::test(flavor = "multi_thread")]
    async fn normalizes_prerelease_without_the_setting() {
        assert_eq!(
            resolve("3.14.0-b.1").await,
            ResolveVersionOutput {
                candidate: Some(UnresolvedVersionSpec::parse("3.14.0-beta.1").unwrap()),
                version: None,
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn normalizes_prerelease_and_appends_build_when_enabled() {
        assert_eq!(
            resolve_with_latest_build("3.14.0-b.1").await,
            ResolveVersionOutput {
                candidate: Some(UnresolvedVersionSpec::parse("3.14.0-beta.1").unwrap()),
                version: Some(VersionSpec::parse("3.14.0-beta.1+20250604").unwrap()),
            }
        );
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
