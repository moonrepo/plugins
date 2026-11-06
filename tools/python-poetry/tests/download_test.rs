use proto_pdk_test_utils::*;

mod python_poetry_tool {
    use super::*;

    generate_native_install_tests!("poetry-test", "2.1.0");

    #[tokio::test(flavor = "multi_thread")]
    async fn reports_installer_errors() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("poetry-test").await;

        let output = plugin
            .native_install(NativeInstallInput {
                context: PluginContext {
                    version: VersionSpec::parse("0.0.1").unwrap(),
                    ..Default::default()
                },
                install_dir: VirtualPath::new(sandbox.path().join("install")),
                ..Default::default()
            })
            .await;

        assert!(!output.installed);
        assert!(
            output
                .error
                .unwrap()
                .contains("Version 0.0.1 does not exist")
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn configures_the_installed_poetry() {
        use std::os::unix::fs::PermissionsExt;

        let sandbox = create_empty_proto_sandbox();
        let tool_dir = sandbox.path().join(".proto/tools/poetry-test/2.1.0");
        let exe_path = tool_dir.join("bin/poetry");

        sandbox.create_file(
            ".proto/tools/poetry-test/2.1.0/bin/poetry",
            "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/args\"\n",
        );
        std::fs::set_permissions(&exe_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let plugin = sandbox.create_plugin("poetry-test").await;

        plugin
            .post_install(InstallHook {
                context: PluginContext {
                    tool_dir: VirtualPath::new(&tool_dir),
                    version: VersionSpec::parse("2.1.0").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            std::fs::read_to_string(tool_dir.join("bin/args")).unwrap(),
            "config virtualenvs.use-poetry-python false\n"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("poetry-test", |config| {
                config.host(HostOS::Linux, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("poetry")
                .unwrap()
                .exe_path,
            Some("bin/poetry".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("poetry-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin
                .locate_executables(LocateExecutablesInput {
                    context: PluginContext {
                        version: VersionSpec::parse("1.2.0").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await
                .exes
                .get("poetry")
                .unwrap()
                .exe_path,
            Some("bin/poetry.exe".into())
        );
    }
}
