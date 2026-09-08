// NOTE: Doesn't work in GitHub CI.
//
// #[cfg(unix)]
// mod ruby_tool {
//     use proto_pdk_test_utils::*;

//     generate_build_install_tests!("ruby-test", "3.4.0");
// }

mod ruby_tool {
    use proto_pdk_test_utils::*;

    async fn get_build_version(version: &str) -> String {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("ruby-test", |config| {
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
            panic!("missing ruby-build command");
        };

        cmd.args[1].clone()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn passes_ruby_version_to_ruby_build() {
        assert_eq!(get_build_version("3.4.5").await, "3.4.5");
        assert_eq!(get_build_version("3.4.5+4").await, "3.4.5");
        assert_eq!(
            get_build_version("4.0.0-preview.2+4").await,
            "4.0.0-preview2"
        );
        assert_eq!(get_build_version("3.4.0-rc.1").await, "3.4.0-rc1");
    }
}
