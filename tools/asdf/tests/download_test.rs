use proto_pdk_test_utils::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

mod asdf_tool {
    use super::*;

    generate_download_install_tests!("asdf-test", "18.0.0");

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_tool_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        // Create plugin with download script
        fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();
        sandbox.create_file(
            ".asdf/plugins/nodejs/bin/download",
            "#!/bin/sh\necho 'Downloading nodejs 18.0.0' > $ASDF_DOWNLOAD_PATH/download.log",
        );
        fs::set_permissions(
            sandbox.path().join(".asdf/plugins/nodejs/bin/download"),
            fs::Permissions::from_mode(0o755),
        ).unwrap();

        let result = plugin.download_prebuilt(DownloadPrebuiltInput {
            context: ToolContext {
                version: VersionSpec::parse("18.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await;
        assert!(!result.download_url.is_empty());
        
        let download_log = fs::read_to_string(
            sandbox.path().join("downloads/nodejs/18.0.0/download.log")
        ).unwrap();
        assert_eq!(download_log.trim(), "Downloading nodejs 18.0.0");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn installs_tool_version() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        // Create plugin with install script
        fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();
        sandbox.create_file(
            ".asdf/plugins/nodejs/bin/install",
            "#!/bin/sh\necho 'Installing nodejs 18.0.0' > $ASDF_INSTALL_PATH/install.log",
        );
        fs::set_permissions(
            sandbox.path().join(".asdf/plugins/nodejs/bin/install"),
            fs::Permissions::from_mode(0o755),
        ).unwrap();

        // Create download path
        fs::create_dir_all(sandbox.path().join("downloads/nodejs/18.0.0")).unwrap();

        let result = plugin.native_install(NativeInstallInput {
            context: ToolContext {
                version: VersionSpec::parse("18.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await;
        assert!(result.installed);
        
        let install_log = fs::read_to_string(
            sandbox.path().join("installs/nodejs/18.0.0/install.log")
        ).unwrap();
        assert_eq!(install_log.trim(), "Installing nodejs 18.0.0");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn handles_download_failure() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        // Create plugin with failing download script
        fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();
        sandbox.create_file(
            ".asdf/plugins/nodejs/bin/download",
            "#!/bin/sh\nexit 1",
        );
        fs::set_permissions(
            sandbox.path().join(".asdf/plugins/nodejs/bin/download"),
            fs::Permissions::from_mode(0o755),
        ).unwrap();

        let result = plugin.download_prebuilt(DownloadPrebuiltInput {
            context: ToolContext {
                version: VersionSpec::parse("18.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await;
        assert!(result.download_url.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn handles_install_failure() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("asdf-test").await;

        // Create plugin with failing install script
        fs::create_dir_all(sandbox.path().join(".asdf/plugins/nodejs/bin")).unwrap();
        sandbox.create_file(
            ".asdf/plugins/nodejs/bin/install",
            "#!/bin/sh\nexit 1",
        );
        fs::set_permissions(
            sandbox.path().join(".asdf/plugins/nodejs/bin/install"),
            fs::Permissions::from_mode(0o755),
        ).unwrap();

        // Create download path
        fs::create_dir_all(sandbox.path().join("downloads/nodejs/18.0.0")).unwrap();

        let result = plugin.native_install(NativeInstallInput {
            context: ToolContext {
                version: VersionSpec::parse("18.0.0").unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }).await;
        assert!(!result.installed);
    }
} 