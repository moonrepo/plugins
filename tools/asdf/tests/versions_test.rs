use proto_pdk::*;
use proto_pdk_test_utils::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

mod asdf_tool {
    use super::*;

    generate_resolve_versions_tests!("asdf-test", {
        "18.0.0" => "18.0.0",
        "17.0.0" => "17.0.0",
        "16.0.0" => "16.0.0",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_scripts() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        // Create plugin with list-all script
        fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();
        sandbox.create_file(
            ".asdf/plugins/nodejs/bin/list-all",
            "#!/bin/sh\necho '18.0.0 17.0.0 16.0.0'",
        );
        fs::set_permissions(
            sandbox.path().join(".asdf/plugins/nodejs/bin/list-all"),
            fs::Permissions::from_mode(0o755),
        ).unwrap();

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        assert!(!output.versions.is_empty());
        assert!(output.versions.contains(&VersionSpec::parse("18.0.0").unwrap()));
        assert!(output.versions.contains(&VersionSpec::parse("17.0.0").unwrap()));
        assert!(output.versions.contains(&VersionSpec::parse("16.0.0").unwrap()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        // Create plugin with latest-stable script
        fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();
        sandbox.create_file(
            ".asdf/plugins/nodejs/bin/latest-stable",
            "#!/bin/sh\necho '18.0.0'",
        );
        fs::set_permissions(
            sandbox.path().join(".asdf/plugins/nodejs/bin/latest-stable"),
            fs::Permissions::from_mode(0o755),
        ).unwrap();

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn detects_version_files() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        let output = plugin.detect_version_files().await;
        assert_eq!(output.files, vec![".tool-versions"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_version_file() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        let output = plugin.parse_version_file(ParseVersionFileInput {
            file: ".tool-versions".into(),
            content: "nodejs 18.0.0".into(),
            ..Default::default()
        }).await;
        assert_eq!(output.version.unwrap().to_string(), "18.0.0");
    }
}

#[tokio::test]
async fn detects_legacy_version_files() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("test").await;

    // Create plugin with legacy version files
    fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();
    sandbox.create_file(
        ".asdf/plugins/nodejs/bin/list-legacy-filenames",
        "#!/bin/sh\necho '.node-version .nvmrc'",
    );
    fs::set_permissions(
        sandbox.path().join(".asdf/plugins/nodejs/bin/list-legacy-filenames"),
        fs::Permissions::from_mode(0o755),
    ).unwrap();

    // Create legacy version file
    sandbox.create_file(".node-version", "18.0.0");

    let result = plugin.detect_version_files().await;
    assert!(result.files.contains(&".node-version".into()));
    assert!(result.files.contains(&".nvmrc".into()));
}

#[tokio::test]
async fn parses_tool_versions() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("test").await;

    // Create .tool-versions file
    sandbox.create_file(
        ".tool-versions",
        "nodejs 18.0.0 16.0.0\nruby 3.2.0\n",
    );

    let version = plugin.parse_version_file(ParseVersionFileInput {
        file: ".tool-versions".into(),
        content: fs::read_to_string(sandbox.path().join(".tool-versions")).unwrap(),
        ..Default::default()
    }).await.version.unwrap();

    assert_eq!(version.to_string(), "18.0.0");
}

#[tokio::test]
async fn parses_legacy_version_file() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("test").await;

    // Create plugin with legacy version parser
    fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();
    sandbox.create_file(
        ".asdf/plugins/nodejs/bin/parse-legacy-file",
        "#!/bin/sh\ncat $1",
    );
    fs::set_permissions(
        sandbox.path().join(".asdf/plugins/nodejs/bin/parse-legacy-file"),
        fs::Permissions::from_mode(0o755),
    ).unwrap();

    // Create legacy version file
    sandbox.create_file(".node-version", "18.0.0");

    let version = plugin.parse_version_file(ParseVersionFileInput {
        file: ".node-version".into(),
        content: fs::read_to_string(sandbox.path().join(".node-version")).unwrap(),
        ..Default::default()
    }).await.version.unwrap();

    assert_eq!(version.to_string(), "18.0.0");
} 