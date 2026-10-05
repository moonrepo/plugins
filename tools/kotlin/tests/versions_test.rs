use proto_pdk_test_utils::*;

generate_resolve_versions_tests!("kotlin-test", {
    "0.6.31" => "0.6.31",
    "0.11.91+1" => "0.11.91+1",
    "1.0.0" => "1.0.0",
    "1.0.1-2" => "1.0.1-2",
    "1.0.5-2" => "1.0.5-2",
    "1.1.0" => "1.1.0",
    "1.1" => "1.1.61",
    "1.1.4-3" => "1.1.4-3",
    "1.2-M1" => "1.2.0-M1",
    "~1.2" => "1.2.71",
    "1.2-beta2" => "1.2.0-beta2",
    "1.3-rc4" => "1.3.0-rc4",
    "1.3.0-rc-190" => "1.3.0-rc-190",
    "1.3.70-eap-274" => "1.3.70-eap-274",
    "1.4.0-rc" => "1.4.0-rc",
    "1.9" => "1.9.25",
    "2.0" => "2.0.21",
    "1.8.22" => "1.8.22",
    "2.4.20-RC" => "2.4.20-RC",
});

#[tokio::test(flavor = "multi_thread")]
async fn loads_stable_and_preview_releases() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("kotlin-test").await;
    let output = plugin.load_versions(LoadVersionsInput::default()).await;

    assert!(
        output
            .versions
            .contains(&VersionSpec::parse("1.9.25").unwrap())
    );
    assert!(
        output
            .versions
            .contains(&VersionSpec::parse("2.4.20-RC").unwrap())
    );
    let latest = output.latest.as_ref().unwrap();
    assert_eq!(output.aliases.get("latest"), Some(latest));
    assert!(!latest.to_string().contains('-'));
}

#[tokio::test(flavor = "multi_thread")]
async fn detects_version_files_in_priority_order() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("kotlin-test").await;
    let output = plugin
        .detect_version_files(DetectVersionInput::default())
        .await;

    assert_eq!(output.files, [".kotlin-version", ".sdkmanrc"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn parses_version_files() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("kotlin-test").await;

    for (file, content, expected) in [
        (
            ".kotlin-version",
            "# compiler\r\n\r\n 2.4.20 \r\n2.0.0",
            Some("2.4.20"),
        ),
        (".kotlin-version", "  ^2.0 # range", Some("^2.0")),
        (".kotlin-version", "latest", Some("latest")),
        (".kotlin-version", "2.4.20-RC", Some("2.4.20-RC")),
        (".kotlin-version", "1.1.4-3", Some("1.1.4-3")),
        (".sdkmanrc", "kotlin=1.3.0-rc-190", Some("1.3.0-rc-190")),
        (".kotlin-version", "# no version\n\n", None),
        (
            ".sdkmanrc",
            "# SDKs\r\njava=21.0.1-tem\r\n kotlin = 1.9.25 # compiler\r\ngradle=8.0",
            Some("1.9.25"),
        ),
        (
            ".sdkmanrc",
            "# kotlin=1.0.0\nkotlin=2.4.20-Beta2\nkotlin=2.0.0",
            Some("2.4.20-Beta2"),
        ),
        (".sdkmanrc", "java=21-tem\nkotlinx=2.0.0\n", None),
        ("build.gradle.kts", "kotlin=2.0.0", None),
    ] {
        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                file: file.into(),
                content: content.into(),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version,
            expected.map(|v| UnresolvedVersionSpec::parse(v).unwrap()),
            "{file}: {content}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_malformed_explicit_versions() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("kotlin-test").await;

    for (file, content) in [
        (".sdkmanrc", "kotlin= # missing"),
        (".kotlin-version", ">>2.0"),
    ] {
        let result = plugin
            .tool
            .plugin
            .call_func_with::<_, _, ParseVersionFileOutput>(
                PluginFunction::ParseVersionFile,
                ParseVersionFileInput {
                    file: file.into(),
                    content: content.into(),
                    ..Default::default()
                },
            )
            .await;
        assert!(result.is_err(), "{file}: {content}");
    }
}
