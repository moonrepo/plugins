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

mod zig_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zig-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("0.14.1")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("zig-x86_64-linux-0.14.1".into()),
                checksum: Some(Checksum::sha256(
                    "24aeeec8af16c381934a6cd7d95c807a8cb2cf7df9fa40d359aa884195c4716c".into(),
                )),
                download_name: Some("zig-x86_64-linux-0.14.1.tar.xz".into()),
                download_url: "https://ziglang.org/download/0.14.1/zig-x86_64-linux-0.14.1.tar.xz"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zig-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("0.14.1")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("zig-aarch64-macos-0.14.1".into()),
                checksum: Some(Checksum::sha256(
                    "39f3dc5e79c22088ce878edc821dedb4ca5a1cd9f5ef915e9b3cc3053e8faefa".into(),
                )),
                download_name: Some("zig-aarch64-macos-0.14.1.tar.xz".into()),
                download_url: "https://ziglang.org/download/0.14.1/zig-aarch64-macos-0.14.1.tar.xz"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x86() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zig-test", |config| {
                config.host(HostOS::Windows, HostArch::X86);
            })
            .await;

        assert_eq!(
            plugin.download_prebuilt(create_input("0.14.1")).await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("zig-x86-windows-0.14.1".into()),
                checksum: Some(Checksum::sha256(
                    "3ee730c2a5523570dc4dc1b724f3e4f30174ebc1fa109ca472a719586a473b18".into(),
                )),
                download_name: Some("zig-x86-windows-0.14.1.zip".into()),
                download_url: "https://ziglang.org/download/0.14.1/zig-x86-windows-0.14.1.zip"
                    .into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn follows_legacy_archive_names_from_index() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zig-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin.download_prebuilt(create_input("0.14.0")).await;

        assert_eq!(
            output.archive_prefix,
            Some("zig-linux-x86_64-0.14.0".into())
        );
        assert_eq!(
            output.download_name,
            Some("zig-linux-x86_64-0.14.0.tar.xz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resolves_master_build() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zig-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin.download_prebuilt(create_input("canary")).await;

        assert!(
            output
                .download_url
                .starts_with("https://ziglang.org/builds/")
        );
        assert!(output.download_name.unwrap().contains("-dev."));
        assert!(output.checksum.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_unix_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zig-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;

        assert_eq!(output.exes["zig"].exe_path, Some("zig".into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("zig-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;

        assert_eq!(output.exes["zig"].exe_path, Some("zig.exe".into()));
    }
}
