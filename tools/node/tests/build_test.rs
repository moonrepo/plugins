// NOTE: Building Node.js takes a very long time!

// use proto_pdk_test_utils::*;

// mod node_tool {
//     use super::*;

//     generate_build_install_tests!("node-test", "22.0.0");
// }

mod node_tool {
    use proto_pdk_test_utils::*;

    async fn get_build_instructions(os: HostOS) -> BuildInstructionsOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("node-test", |config| {
                config.host(os, HostArch::X64);
            })
            .await;

        plugin
            .tool
            .plugin
            .call_func_with(
                PluginFunction::BuildInstructions,
                BuildInstructionsInput {
                    context: PluginContext {
                        version: VersionSpec::parse("24.22.2").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn installs_prebuilt_layout_on_unix() {
        let output = get_build_instructions(HostOS::Linux).await;

        assert_eq!(
            output.instructions[2..],
            [
                BuildInstruction::RunCommand(Box::new(CommandInstruction::new(
                    "make",
                    ["install", "DESTDIR=out/install", "PREFIX=/"],
                ))),
                BuildInstruction::RemoveAllExcept(vec!["out".into()]),
                BuildInstruction::MoveFile("out/install/bin".into(), "bin".into()),
                BuildInstruction::MoveFile("out/install/include".into(), "include".into()),
                BuildInstruction::MoveFile("out/install/lib".into(), "lib".into()),
                BuildInstruction::RemoveDir("out".into()),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn uses_default_compiler_on_unix() {
        let output = get_build_instructions(HostOS::MacOS).await;

        // macOS requires the Xcode toolchain, not GCC
        assert!(
            !output
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, BuildInstruction::SetEnvVar(..)))
        );
        assert!(
            output
                .requirements
                .contains(&BuildRequirement::CommandExistsOnPath("python3".into()))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn moves_exe_to_root_on_windows() {
        let output = get_build_instructions(HostOS::Windows).await;

        assert_eq!(
            output.instructions[1..],
            [
                BuildInstruction::MoveFile("out/Release/node.exe".into(), "node.exe".into()),
                BuildInstruction::RemoveAllExcept(vec!["node.exe".into()]),
            ]
        );
    }
}
