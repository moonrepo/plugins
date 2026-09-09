#[cfg(unix)]
mod python_tool {
    use proto_pdk_test_utils::*;

    generate_build_install_tests!("python-test", "3.12.0");

    async fn get_build_version(version: &str) -> String {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output: BuildInstructionsOutput = plugin
            .tool
            .plugin
            .call_func_with(
                PluginFunction::BuildInstructions,
                BuildInstructionsInput {
                    context: PluginContext {
                        version: VersionSpec::parse(version).unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let Some(BuildInstruction::RunCommand(cmd)) = output.instructions.last() else {
            panic!("missing python-build command");
        };

        cmd.args[1].clone()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn passes_python_version_to_python_build() {
        assert_eq!(get_build_version("3.12.0").await, "3.12.0");
        assert_eq!(get_build_version("3.12.0+20231002").await, "3.12.0");
        assert_eq!(get_build_version("3.14.0-alpha.7").await, "3.14.0a7");
        assert_eq!(
            get_build_version("3.14.0-beta.1+20250604").await,
            "3.14.0b1"
        );
        assert_eq!(get_build_version("3.14.0-rc.1").await, "3.14.0rc1");
    }
}
