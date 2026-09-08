use proto_pdk_test_utils::*;

fn create_input(version: &str) -> ResolveVersionInput {
    ResolveVersionInput {
        initial: UnresolvedVersionSpec::parse(version).unwrap(),
        ..Default::default()
    }
}

mod ruby_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn appends_latest_build_to_full_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("ruby-test").await;

        assert_eq!(
            plugin.resolve_version(create_input("3.4.5")).await,
            ResolveVersionOutput {
                version: Some(VersionSpec::parse("3.4.5+4").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn appends_latest_build_to_prerelease() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("ruby-test").await;

        assert_eq!(
            plugin
                .resolve_version(create_input("4.0.0-preview.3"))
                .await,
            ResolveVersionOutput {
                version: Some(VersionSpec::parse("4.0.0-preview.3+4").unwrap()),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_explicit_build() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("ruby-test").await;

        assert_eq!(
            plugin.resolve_version(create_input("3.4.5+2")).await,
            ResolveVersionOutput::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_unknown_release() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("ruby-test").await;

        assert_eq!(
            plugin.resolve_version(create_input("3.2.0")).await,
            ResolveVersionOutput::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_ranges_and_aliases() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("ruby-test").await;

        for spec in ["3.4", "^3.4.5", "latest"] {
            assert_eq!(
                plugin.resolve_version(create_input(spec)).await,
                ResolveVersionOutput::default(),
                "for {spec}"
            );
        }
    }
}
