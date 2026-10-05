use proto_pdk_test_utils::*;
use std::path::PathBuf;

mod kotlin_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_legacy_and_modern_launchers_on_unix_and_windows() {
        for os in [HostOS::Linux, HostOS::MacOS, HostOS::Windows] {
            for modern in [false, true] {
                let sandbox = create_empty_proto_sandbox();
                let plugin = sandbox
                    .create_plugin_with_config("kotlin-test", |config| {
                        config.host(os, HostArch::X64);
                    })
                    .await;
                let suffix = if os.is_windows() { ".bat" } else { "" };
                let mut names = vec!["kotlin", "kotlinc", "kotlinc-jvm", "kotlinc-js", "kapt"];
                if modern {
                    names.extend(["kotlinr", "kotlinc-wasm"]);
                }
                for name in &names {
                    sandbox.create_file(
                        format!("install with spaces/bin/{name}{suffix}"),
                        "launcher",
                    );
                }

                let output = plugin
                    .locate_executables(LocateExecutablesInput {
                        install_dir: VirtualPath::new(sandbox.path().join("install with spaces")),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(output.exes.len(), names.len());
                assert_eq!(output.exes_dirs, [PathBuf::from("bin")]);
                assert!(output.globals_lookup_dirs.is_empty());
                for name in &names {
                    let exe = &output.exes[*name];
                    let target = if *name == "kotlin" && modern {
                        "kotlinr"
                    } else {
                        name
                    };
                    assert_eq!(exe.exe_path, Some(format!("bin/{target}{suffix}").into()));
                    assert_eq!(exe.primary, *name == "kotlin");
                    assert!(exe.no_bin);
                    assert!(!exe.no_shim);
                    assert_eq!(exe.update_perms, !os.is_windows());
                }
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn omits_optional_launchers_when_absent() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("kotlin-test").await;
        let output = plugin
            .locate_executables(LocateExecutablesInput {
                install_dir: VirtualPath::new(sandbox.path()),
                ..Default::default()
            })
            .await;

        assert_eq!(output.exes.len(), 2);
        assert!(output.exes.contains_key("kotlin"));
        assert!(output.exes.contains_key("kotlinc"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_compiler_only_historical_archives() {
        for os in [HostOS::Linux, HostOS::MacOS, HostOS::Windows] {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("kotlin-test", |config| {
                    config.host(os, HostArch::X64);
                })
                .await;
            let suffix = if os.is_windows() { ".bat" } else { "" };
            for name in ["kotlinc-jvm", "kotlinc-js"] {
                sandbox.create_file(format!("install/bin/{name}{suffix}"), "launcher");
            }
            let output = plugin
                .locate_executables(LocateExecutablesInput {
                    install_dir: VirtualPath::new(sandbox.path().join("install")),
                    ..Default::default()
                })
                .await;

            for name in ["kotlin", "kotlinc", "kotlinc-jvm"] {
                assert_eq!(
                    output.exes[name].exe_path,
                    Some(format!("bin/kotlinc-jvm{suffix}").into())
                );
            }
            assert_eq!(output.exes.len(), 4);
            assert!(output.exes["kotlin"].primary);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn activates_the_selected_installation_without_overriding_java() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("kotlin-test").await;

        for version in ["1.8.22", "2.4.20"] {
            let directory = sandbox
                .path()
                .join(format!("install with spaces/{version}"));
            let output = plugin
                .activate_environment(ActivateEnvironmentInput {
                    context: PluginContext {
                        tool_dir: VirtualPath::new(&directory),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await;

            assert_eq!(output.env.len(), 1);
            assert_eq!(PathBuf::from(&output.env["KOTLIN_HOME"]), directory);
            assert!(output.paths.is_empty());
        }
    }

    #[cfg(not(windows))]
    #[tokio::test(flavor = "multi_thread")]
    async fn creates_shims_for_installed_launchers() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("kotlin-test").await;
        let spec = ToolSpec::new_resolved(VersionSpec::parse("2.4.20").unwrap());
        let directory = plugin.tool.get_product_dir(&spec);
        std::fs::create_dir_all(directory.join("bin")).unwrap();
        let names = [
            "kotlin",
            "kotlinr",
            "kotlinc",
            "kotlinc-jvm",
            "kotlinc-js",
            "kotlinc-wasm",
            "kapt",
        ];
        for name in names {
            std::fs::write(directory.join("bin").join(name), "#!/bin/sh\n").unwrap();
        }

        flow::link::Linker::new(&plugin.tool, &spec)
            .unwrap()
            .link_shims(false)
            .await
            .unwrap();

        for name in names {
            assert!(sandbox.proto_dir.join("shims").join(name).exists());
        }
        let bins = flow::locate::Locator::new(&plugin.tool, &spec)
            .locate_bins(None)
            .await
            .unwrap();
        assert!(bins.is_empty());
    }
}
