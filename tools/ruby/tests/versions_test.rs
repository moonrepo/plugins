use proto_pdk_test_utils::*;

mod ruby_tool {
    use super::*;
    use ::ruby_tool::RubyToolConfig;

    fn create_input(version: &str) -> ResolveVersionInput {
        ResolveVersionInput {
            initial: UnresolvedVersionSpec::parse(version).unwrap(),
            ..Default::default()
        }
    }

    // The registry versions are filtered by host, and Ruby has no
    // Windows artifacts, so pin the host instead of inheriting it
    fn use_linux_host(host: &mut HostEnvironment) {
        host.os = HostOS::Linux;
        host.arch = HostArch::X64;
        host.libc = HostLibc::Gnu;
    }

    async fn load_version_strings() -> Vec<String> {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host_with(use_linux_host);
            })
            .await;

        plugin
            .load_versions(LoadVersionsInput::default())
            .await
            .versions
            .iter()
            .map(|v| v.to_string())
            .collect()
    }

    async fn resolve(version: &str) -> ResolveVersionOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("ruby-test").await;

        plugin.resolve_version(create_input(version)).await
    }

    async fn resolve_with_latest_build(version: &str) -> ResolveVersionOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.tool_config(RubyToolConfig {
                    use_latest_build: true,
                });
            })
            .await;

        plugin.resolve_version(create_input(version)).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_prereleases_in_registry_format() {
        let versions = load_version_strings().await;

        assert!(versions.contains(&"3.4.5".to_string()));
        assert!(versions.contains(&"4.0.0-preview.2".to_string()));
        assert!(versions.contains(&"3.4.0-rc.1".to_string()));
        assert!(!versions.contains(&"4.0.0-preview2".to_string()));
    }

    // These are our own builds, so they only exist in the registry
    #[tokio::test(flavor = "multi_thread")]
    async fn includes_build_specific_versions() {
        let versions = load_version_strings().await;

        assert!(versions.contains(&"3.4.5+4".to_string()));
        assert!(versions.contains(&"3.4.5+1".to_string()));
        assert!(versions.contains(&"4.0.0-preview.2+4".to_string()));
    }

    // Build specific versions are part of the version list now,
    // so ranges resolve to the latest build instead of the plain version
    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_range_to_build_specific_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
                config.host_with(use_linux_host);
            })
            .await;
        let mut spec = ToolSpec::parse("3.4").unwrap();

        flow::resolve::Resolver::new(&plugin.tool)
            .resolve_version(&mut spec, false)
            .await
            .unwrap();

        let resolved = spec.get_resolved_version().to_string();

        assert!(resolved.starts_with("3.4."), "got {resolved}");
        assert!(resolved.contains('+'), "got {resolved}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_append_build_by_default() {
        assert_eq!(resolve("3.4.5").await, ResolveVersionOutput::default());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn appends_latest_build_when_enabled() {
        assert_eq!(
            resolve_with_latest_build("3.4.5").await,
            ResolveVersionOutput {
                version: Some(VersionSpec::parse("3.4.5+4").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn appends_latest_build_to_prerelease_when_enabled() {
        assert_eq!(
            resolve_with_latest_build("4.0.0-preview.3").await,
            ResolveVersionOutput {
                version: Some(VersionSpec::parse("4.0.0-preview.3+4").unwrap()),
                ..Default::default()
            }
        );
    }

    // An explicit build always wins over the setting
    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_explicit_build_when_enabled() {
        assert_eq!(
            resolve_with_latest_build("3.4.5+2").await,
            ResolveVersionOutput::default()
        );
    }

    // Ranges and aliases are resolved from the version list instead
    #[tokio::test(flavor = "multi_thread")]
    async fn skips_ranges_and_aliases_when_enabled() {
        for spec in ["3.4", "^3.4.5", "latest"] {
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
            resolve("4.0.0-preview2").await,
            ResolveVersionOutput {
                candidate: Some(UnresolvedVersionSpec::parse("4.0.0-preview.2").unwrap()),
                version: None,
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn normalizes_prerelease_and_appends_build_when_enabled() {
        assert_eq!(
            resolve_with_latest_build("4.0.0-preview2").await,
            ResolveVersionOutput {
                candidate: Some(UnresolvedVersionSpec::parse("4.0.0-preview.2").unwrap()),
                version: Some(VersionSpec::parse("4.0.0-preview.2+4").unwrap()),
            }
        );
    }
}
