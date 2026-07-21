use proto_pdk_test_utils::*;

mod java_tool {
    use super::*;

    // These all resolve against the default openjdk distribution, whose
    // jdk.java.net streams are frozen, so they never drift. All cases in
    // this macro share a plugin instance (and its version cache), so only
    // same-scoped versions may be used!
    generate_resolve_versions_tests!("java-test", {
        "17" => "open-jdk-17.0.2+8",
        "19" => "open-jdk-19.0.2+7",
        "open-jdk-21" => "open-jdk-21.0.2+13",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_vendor_scoped_versions() {
        // Vendor scoped versions drift with quarterly CPU releases,
        // and will need to be updated over time. Each case creates a
        // fresh plugin, as the remote version cache is per-instance
        // and the version list differs per distribution scope. The host
        // is pinned since not all vendors ship all platforms (temurin 8
        // has no macos-arm64 build, for example).
        for (requested, expected) in [
            ("temurin-8", "temurin-8.0.492+9"),
            ("temurin-21", "temurin-21.0.11+10"),
            ("zulu-11", "zulu-11.0.31+11"),
        ] {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;
            let mut spec = ToolSpec::parse(requested).unwrap();

            flow::resolve::Resolver::new(&plugin.tool)
                .resolve_version(&mut spec, false)
                .await
                .unwrap();

            assert_eq!(spec.get_resolved_version(), expected, "for {requested}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_scoped_by_distribution() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .load_versions(LoadVersionsInput {
                initial: UnresolvedVersionSpec::parse("zulu-21").unwrap(),
                ..Default::default()
            })
            .await;

        assert!(!output.versions.is_empty());
        assert!(
            output
                .versions
                .iter()
                .all(|version| version.get_scope() == Some("zulu"))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_all_distributions_when_unscoped() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        let scopes = output
            .versions
            .iter()
            .filter_map(|version| version.get_scope())
            .collect::<std::collections::HashSet<_>>();

        // Unscoped listings query every supported distribution
        assert!(scopes.len() > 3);
        assert!(scopes.contains("open-jdk"));
        assert!(scopes.contains("temurin"));
        assert!(scopes.contains("zulu"));
        // Unknown foojay distributions must be skipped, not errors
        assert!(!scopes.contains("eliya"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn injects_default_distribution_scope() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .resolve_version(ResolveVersionInput {
                initial: UnresolvedVersionSpec::parse("21").unwrap(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.candidate,
            Some(UnresolvedVersionSpec::parse("open-jdk-21").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_scoping_aliases() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .resolve_version(ResolveVersionInput {
                initial: UnresolvedVersionSpec::parse("latest").unwrap(),
                ..Default::default()
            })
            .await;

        assert_eq!(output.candidate, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn keeps_explicit_distribution_scope() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .resolve_version(ResolveVersionInput {
                initial: UnresolvedVersionSpec::parse("zulu-21").unwrap(),
                ..Default::default()
            })
            .await;

        assert_eq!(output.candidate, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "fake")]
    async fn errors_invalid_distribution_scope() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        plugin
            .resolve_version(ResolveVersionInput {
                initial: UnresolvedVersionSpec::parse("fake-21").unwrap(),
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_java_version_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "21.0.11\n".into(),
                file: ".java-version".into(),
                ..Default::default()
            })
            .await;

        // Unscoped versions receive the default distribution
        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("open-jdk-21.0.11").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_java_version_file_keeps_existing_scope() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "temurin-21\n".into(),
                file: ".java-version".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("temurin-21").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_java_version_file_keeps_aliases() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "latest\n".into(),
                file: ".java-version".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("latest").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_sdkmanrc_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "java=21.0.11-tem\n".into(),
                file: ".sdkmanrc".into(),
                ..Default::default()
            })
            .await;

        // SDKMAN vendor suffixes map to distribution scopes
        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("temurin-21.0.11").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_sdkmanrc_vendor_suffixes() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        for (vendor, dist) in [
            ("tem", "temurin"),
            ("zulu", "zulu"),
            ("amzn", "corretto"),
            ("librca", "liberica"),
            ("ms", "microsoft"),
            ("sem", "semeru"),
            ("sapmchn", "sap-machine"),
            ("nik", "liberica-native"),
            ("graalce", "graalvm-community"),
            ("graal", "graalvm"),
            ("albba", "dragonwell"),
            ("open", "open-jdk"),
            ("oracle", "oracle"),
        ] {
            let output = plugin
                .parse_version_file(ParseVersionFileInput {
                    content: format!("java=21.0.11-{vendor}\n"),
                    file: ".sdkmanrc".into(),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.version.unwrap(),
                UnresolvedVersionSpec::parse(format!("{dist}-21.0.11")).unwrap(),
                "for {vendor}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parse_sdkmanrc_without_vendor() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                content: "java=21\n".into(),
                file: ".sdkmanrc".into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version.unwrap(),
            UnresolvedVersionSpec::parse("open-jdk-21").unwrap()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "foobar")]
    async fn errors_unknown_sdkman_vendor() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("java-test").await;

        plugin
            .parse_version_file(ParseVersionFileInput {
                content: "java=21.0.11-foobar\n".into(),
                file: ".sdkmanrc".into(),
                ..Default::default()
            })
            .await;
    }
}
