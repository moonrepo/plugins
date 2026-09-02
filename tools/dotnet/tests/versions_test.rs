use proto_pdk_test_utils::*;

mod dotnet_tool {
    use super::*;

    // Expectations are taken from channels that no longer receive releases, so
    // they cannot drift: 7.0 and 3.1 are both end-of-life.
    generate_resolve_versions_tests!("dotnet-test", {
        "8.0.404" => "8.0.404",
        "3.1.426" => "3.1.426",
        "7.0" => "7.0.410",
        // The forms the `global.json` mapping emits. A compound range is how a
        // feature band is expressed, so it has to resolve as well as parse.
        // 7.0.1xx is the case that pins this down: the channel's latest is
        // 7.0.410, so an upper bound that was dropped or ignored would resolve
        // there instead of to the 1xx band's own latest.
        ">=7.0.100 && <7.0.200" => "7.0.120",
        ">=7.0.400 && <7.0.500" => "7.0.410",
        ">=3.1.400 && <3.1.500" => "3.1.426",
        "~7.0.400" => "7.0.410",
        "^7.0.400" => "7.0.410",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_release_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn includes_every_feature_band() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        let versions = output
            .versions
            .iter()
            .map(|version| version.to_string())
            .collect::<Vec<_>>();

        // Bands are parallel product lines, and a release lists several under
        // `sdks[]`. Reading only the headline `sdk` — or resolving from git
        // tags — silently loses them.
        // One from each of the 8.0 channel's four bands.
        for version in ["8.0.424", "8.0.319", "8.0.206", "8.0.130"] {
            assert!(
                versions.iter().any(|v| v == version),
                "expected {version} in the loaded versions"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn includes_prereleases() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(
            output
                .versions
                .iter()
                .any(|version| version.to_string().contains("-preview")),
            "expected at least one preview SDK"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_release_cadence_aliases() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        // .NET's own cadence names. proto cannot infer either of them.
        assert!(output.aliases.contains_key("lts"));
        assert!(output.aliases.contains_key("sts"));
        assert!(output.aliases.contains_key("latest"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_aliases_from_the_index_alone() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        // Each alias names a channel, and the index carries every channel's
        // `latest-sdk`, so one request answers it instead of fourteen.
        for alias in ["lts", "sts", "latest", "preview"] {
            let output = plugin
                .resolve_version(ResolveVersionInput {
                    initial: UnresolvedVersionSpec::Alias(alias.into()),
                    ..Default::default()
                })
                .await;

            let version = output
                .version
                .unwrap_or_else(|| panic!("{alias} resolved to nothing"));

            assert!(
                version.as_version().is_some(),
                "{alias} resolved to {version}, not a version"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn leaves_requirements_for_the_listing_to_resolve() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        // A range has to be matched against the real version list, so this must
        // not answer it — returning a version here would skip that entirely.
        // An exact version is left alone for the same reason: `proto install`
        // asks for it to be validated against the list before downloading.
        for spec in ["~8.0.404", "8.0.404"] {
            let output = plugin
                .resolve_version(ResolveVersionInput {
                    initial: UnresolvedVersionSpec::parse(spec).unwrap(),
                    ..Default::default()
                })
                .await;

            assert!(output.version.is_none(), "{spec} was answered early");
        }
    }
}
