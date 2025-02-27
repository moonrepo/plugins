use proto_pdk_test_utils::*;

mod python_poetry_tool {
    use super::*;

    generate_native_install_tests!("poetry-test", "2.1.0");

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
                    context: ToolContext {
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
                    context: ToolContext {
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
