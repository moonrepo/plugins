use proto_pdk_test_utils::*;

mod go_tool {
    use super::*;

    generate_resolve_versions_tests!("go-test", {
        "1.19" => "1.19.13",
        "1.11" => "1.11.13",
        "1.9.0-rc2" => "1.9.0-rc2",
        "1.21.0" => "1.21.0",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_gomod_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: r#"
module github.com/moonrepo/go-plugin

go 1.20

require (
    github.com/99designs/gqlgen v0.17.25
)"#
                .into(),
                file: "go.mod".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("^1.20.0").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_no_version_from_gomod() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: r#"
module github.com/moonrepo/go-plugin

require (
    github.com/99designs/gqlgen v0.17.25
)"#
                .into(),
                file: "go.mod".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(output.version, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_gowork_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: r#"
go 1.18

use (
    ./hello
    ./example
)"#
                .into(),
                file: "go.work".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("^1.18.0").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_no_version_from_gowork() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("go-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: r#"
use (
    ./hello
    ./example
)"#
                .into(),
                file: "go.work".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(output.version, None);
    }
}
