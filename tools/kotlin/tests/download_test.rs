use kotlin_tool::KotlinToolConfig;
use proto_pdk_test_utils::*;

fn input(version: &str) -> DownloadPrebuiltInput {
    DownloadPrebuiltInput {
        context: PluginContext {
            version: VersionSpec::parse(version).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn downloads_the_same_archive_on_supported_platforms() {
    for (os, arch) in [
        (HostOS::Linux, HostArch::X64),
        (HostOS::Linux, HostArch::Arm64),
        (HostOS::MacOS, HostArch::X64),
        (HostOS::MacOS, HostArch::Arm64),
        (HostOS::Windows, HostArch::X64),
        (HostOS::Windows, HostArch::Arm64),
    ] {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("kotlin-test", |config| {
                config.host(os, arch);
            })
            .await;

        assert_eq!(plugin.download_prebuilt(input("2.4.20")).await, DownloadPrebuiltOutput {
            archive_prefix: Some("kotlinc".into()),
            download_name: Some("kotlin-compiler-2.4.20.zip".into()),
            download_url: "https://github.com/JetBrains/kotlin/releases/download/v2.4.20/kotlin-compiler-2.4.20.zip".into(),
            checksum_url: Some("https://github.com/JetBrains/kotlin/releases/download/v2.4.20/kotlin-compiler-2.4.20.zip.sha256".into()),
            ..Default::default()
        });
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn handles_legacy_checksums_and_preserves_prerelease_names() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("kotlin-test").await;

    for (version, checksum) in [
        ("1.3.0", false),
        ("1.8.22", false),
        ("1.9.0-Beta", false),
        ("1.9.0-RC", true),
        ("1.9.0", true),
        ("2.4.20-Beta2", true),
        ("2.4.20-RC", true),
    ] {
        let output = plugin.download_prebuilt(input(version)).await;
        assert!(
            output
                .download_url
                .ends_with(&format!("/v{version}/kotlin-compiler-{version}.zip"))
        );
        assert_eq!(output.checksum_url.is_some(), checksum, "{version}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn uses_custom_distribution_url_for_archive_and_checksum() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox
        .create_plugin_with_config("kotlin-test", |config| {
            config.tool_config(KotlinToolConfig {
                dist_url: "https://mirror.example/kotlin/{version}/{file}".into(),
            });
        })
        .await;
    let output = plugin.download_prebuilt(input("2.4.20")).await;

    assert_eq!(
        output.download_url,
        "https://mirror.example/kotlin/2.4.20/kotlin-compiler-2.4.20.zip"
    );
    assert_eq!(
        output.checksum_url,
        Some(format!("{}.sha256", output.download_url))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_canary_unresolved_and_scoped_versions() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("kotlin-test").await;

    for (version, message) in [
        ("canary", "canary"),
        ("latest", "resolved, unscoped"),
        ("vendor-2.4.20", "resolved, unscoped"),
    ] {
        let error = plugin
            .tool
            .plugin
            .call_func_with::<_, _, DownloadPrebuiltOutput>(
                PluginFunction::DownloadPrebuilt,
                input(version),
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
}
