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

        let error = output.error.unwrap_or_default();

        assert!(!output.installed);
        assert!(error.contains("Version 0.0.1 does not exist"), "{error}");
    }

    #[cfg(unix)]
    async fn run_post_install(version: &str) -> Option<String> {
        use std::os::unix::fs::PermissionsExt;

        let sandbox = create_empty_proto_sandbox();
        let tool_dir = sandbox
            .path()
            .join(format!(".proto/tools/poetry-test/{version}"));
        let exe_path = tool_dir.join("bin/poetry");

        sandbox.create_file(
            format!(".proto/tools/poetry-test/{version}/bin/poetry"),
            "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/args\"\n",
        );
        std::fs::set_permissions(&exe_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let plugin = sandbox.create_plugin("poetry-test").await;

        plugin
            .post_install(InstallHook {
                context: PluginContext {
                    tool_dir: VirtualPath::new(&tool_dir),
                    version: VersionSpec::parse(version).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        std::fs::read_to_string(tool_dir.join("bin/args")).ok()
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn configures_the_installed_poetry_v1() {
        assert_eq!(
            run_post_install("1.8.0").await.as_deref(),
            Some("config virtualenvs.prefer-active-python true\n")
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_configure_poetry_v2() {
        assert_eq!(run_post_install("2.1.0").await, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("poetry-test", |config| {
                config.host(HostPlatform::new(HostOS::Linux, HostArch::Arm64));
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
                config.host(HostPlatform::new(HostOS::Windows, HostArch::X64));
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
