// NOTE: This is very slow in CI, over 30 minutes!

// use proto_pdk_test_utils::*;

// mod moon_tool {
//     use super::*;

//     generate_build_install_tests!("moon-test", "1.30.0");
// }

mod moon_tool {
    use proto_pdk_test_utils::*;

    async fn get_build_instructions(version: &str) -> BuildInstructionsOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("moon-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        plugin
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
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn builds_moon_and_moonx_for_v2() {
        let output = get_build_instructions("2.6.5").await;

        let Some(BuildInstruction::RunCommand(cmd)) = output.instructions.first() else {
            panic!("missing cargo command");
        };

        assert_eq!(
            cmd.args,
            ["build", "--bin", "moon", "--bin", "moonx", "--release"]
        );
        assert_eq!(
            output.instructions.last(),
            Some(&BuildInstruction::RemoveAllExcept(vec![
                "moon".into(),
                "moonx".into()
            ]))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn builds_only_moon_for_v1() {
        let output = get_build_instructions("1.41.8").await;

        let Some(BuildInstruction::RunCommand(cmd)) = output.instructions.first() else {
            panic!("missing cargo command");
        };

        assert_eq!(cmd.args, ["build", "--bin", "moon", "--release"]);
        assert_eq!(
            output.instructions.last(),
            Some(&BuildInstruction::RemoveAllExcept(vec!["moon-test".into()]))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn requires_protoc_from_v2_2() {
        assert!(
            get_build_instructions("2.1.4")
                .await
                .system_dependencies
                .is_empty()
        );
        assert_eq!(
            get_build_instructions("2.2.0")
                .await
                .system_dependencies
                .len(),
            2
        );
    }
}
