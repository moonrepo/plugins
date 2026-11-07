#[cfg(unix)]
mod python_tool {
    use proto_pdk_test_utils::*;

    generate_build_install_tests!("python-test", "3.12.0");

    async fn get_build_output(version: &str) -> BuildInstructionsOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("python-test", |config| {
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

    async fn get_build_version(version: &str) -> String {
        let output = get_build_output(version).await;

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

    // https://github.com/pyenv/pyenv/wiki#suggested-build-environment
    #[tokio::test(flavor = "multi_thread")]
    async fn includes_suggested_system_dependencies() {
        let output = get_build_output("3.14.0").await;
        let get_deps = |pm: HostPackageManager| {
            output
                .system_dependencies
                .iter()
                .find_map(|dep| match dep {
                    SystemDependency::Config(config) if config.manager == Some(pm) => {
                        match &config.dep {
                            DependencyName::Multiple(names) => Some(names.clone()),
                            _ => None,
                        }
                    }
                    _ => None,
                })
                .unwrap()
        };

        // Required for the `compression.zstd` module in 3.14+
        assert!(get_deps(HostPackageManager::Apt).contains(&"libzstd-dev".into()));

        // Alpine doesn't ship bash, which python-build requires
        assert!(get_deps(HostPackageManager::Apk).contains(&"bash".into()));
    }
}
