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
    async fn loads_versions_from_plugin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registers_tool() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("asdf").await;

    let output = plugin.register_tool(ToolMetadataInput::default()).await;
    assert_eq!(output.name, "asdf");
    assert_eq!(output.type_of, PluginType::Language);
    assert!(output.minimum_proto_version.is_some());
    assert!(output.plugin_version.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn lists_all_tool_versions() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("asdf").await;

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
    assert_eq!(output.versions.len(), 3);
    assert!(output.versions.contains(&VersionSpec::parse("18.0.0").unwrap()));
    assert!(output.versions.contains(&VersionSpec::parse("17.0.0").unwrap()));
    assert!(output.versions.contains(&VersionSpec::parse("16.0.0").unwrap()));
}

#[tokio::test(flavor = "multi_thread")]
async fn gets_latest_stable_version() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("asdf").await;

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
    assert_eq!(output.latest.unwrap(), VersionSpec::parse("18.0.0").unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn handles_missing_latest_stable() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("asdf").await;

    // Create plugin without latest-stable script
    fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();

    let output = plugin.load_versions(LoadVersionsInput::default()).await;
    assert!(output.latest.is_none());
} 