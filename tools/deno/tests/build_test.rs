// NOTE: Building Deno takes a very long time!

// use proto_pdk_test_utils::*;

// mod deno_tool {
//     use super::*;

//     generate_build_install_tests!("deno-test", "2.0.0");
// }

mod deno_tool {
    use proto_pdk_test_utils::*;

    async fn get_build_instructions(os: HostOS) -> BuildInstructionsOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("deno-test", |config| {
                config.host(HostPlatform::new(os, HostArch::X64));
            })
            .await;

        plugin
            .tool
            .plugin
            .call_func_with(
                PluginFunction::BuildInstructions,
                BuildInstructionsInput {
                    context: PluginContext {
                        version: VersionSpec::parse("2.9.8").unwrap(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn builds_from_source_archive_with_submodules() {
        let output = get_build_instructions(HostOS::Linux).await;

        assert_eq!(
            output.source,
            Some(SourceLocation::Archive(ArchiveSource {
                url: "https://github.com/denoland/deno/releases/download/v2.9.8/deno_src.tar.gz"
                    .into(),
                prefix: Some("deno".into()),
            }))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn installs_llvm_as_system_dependencies_on_linux() {
        let output = get_build_instructions(HostOS::Linux).await;

        for name in ["clang", "lld"] {
            assert!(
                output
                    .system_dependencies
                    .contains(&SystemDependency::for_os(HostOS::Linux, name)),
                "{name}"
            );
        }

        // The llvm.sh script must be ran as root
        assert!(
            !output
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, BuildInstruction::RequestScript(_)))
        );

        // Only required for running WPT tests
        assert!(!output.requirements.iter().any(
            |req| matches!(req, BuildRequirement::CommandVersion(cmd, _, _) if cmd == "python")
        ));
    }
}
