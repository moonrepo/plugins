use proto_pdk_test_utils::*;
use std::path::PathBuf;

mod java_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_linux_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("java-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("21.0.11").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.download_url,
            "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.11%2B10/OpenJDK21U-jdk_x64_linux_hotspot_21.0.11_10.tar.gz"
        );
        assert_eq!(
            output.download_name,
            Some("OpenJDK21U-jdk_x64_linux_hotspot_21.0.11_10.tar.gz".into())
        );
        assert!(output.checksum.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_macos_arm64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("java-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("21.0.11").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.download_url,
            "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.11%2B10/OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.11_10.tar.gz"
        );
        assert_eq!(
            output.download_name,
            Some("OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.11_10.tar.gz".into())
        );
        assert!(output.checksum.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_windows_x64() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("java-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("21.0.9").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.download_url,
            "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.9%2B10/OpenJDK21U-jdk_x64_windows_hotspot_21.0.9_10.zip"
        );
        assert_eq!(
            output.download_name,
            Some("OpenJDK21U-jdk_x64_windows_hotspot_21.0.9_10.zip".into())
        );
        assert!(output.checksum.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_default_bin() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("java-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput {
                context: PluginContext {
                    version: VersionSpec::parse("21.0.11").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.exes.get("java").unwrap().exe_path,
            Some("bin/java".into())
        );
        assert_eq!(
            output.exes.get("javac").unwrap().exe_path,
            Some("bin/javac".into())
        );
        assert_eq!(output.exes_dirs, vec![PathBuf::from("bin")]);
    }
}
