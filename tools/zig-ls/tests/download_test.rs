use proto_pdk_test_utils::*;

fn create_input(version: &str) -> DownloadPrebuiltInput {
    DownloadPrebuiltInput {
        context: PluginContext {
            version: VersionSpec::parse(version).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}

mod zig_ls_tool {
    use super::*;

    generate_download_install_tests!("zls-test", "0.16.0");

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zls-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("0.16.0")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("zls-x86_64-linux-0.16.0".into()),
                checksum: Some(Checksum::sha256(
                    "ded6d562a0b86ee878b1ddf70ffab2797ce3cdca3b02d6077548f9d56dff96b6".into(),
                )),
                download_name: Some("zls-x86_64-linux-0.16.0.tar.xz".into()),
                download_url: "https://builds.zigtools.org/zls-x86_64-linux-0.16.0.tar.xz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zls-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("0.16.0")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("zls-aarch64-macos-0.16.0".into()),
                checksum: Some(Checksum::sha256(
                    "b93ec549f8558a7e85984a840e9276d274f1059b54ade4254296ef4982958359".into(),
                )),
                download_name: Some("zls-aarch64-macos-0.16.0.tar.xz".into()),
                download_url: "https://builds.zigtools.org/zls-aarch64-macos-0.16.0.tar.xz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x86() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zls-test", |config| {
                config.host(HostOS::Windows, HostArch::X86);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("0.16.0")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("zls-x86-windows-0.16.0".into()),
                checksum: Some(Checksum::sha256(
                    "ecb2870979b35143aa5e7ce92d3b69362a76fd7126c8f950a5f8a7f99a77416f".into(),
                )),
                download_name: Some("zls-x86-windows-0.16.0.zip".into()),
                download_url: "https://builds.zigtools.org/zls-x86-windows-0.16.0.zip".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn follows_legacy_archive_names_from_index() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zls-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin.download_prebuilt(create_input("0.14.0")).await;

        assert_eq!(
            output.archive_prefix,
            Some("zls-linux-x86_64-0.14.0".into())
        );
        assert_eq!(
            output.download_name,
            Some("zls-linux-x86_64-0.14.0.tar.xz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zls-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;

        assert_eq!(output.exes["zls"].exe_path, Some("zls".into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zls-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;

        assert_eq!(output.exes["zls"].exe_path, Some("zls.exe".into()));
    }
}
