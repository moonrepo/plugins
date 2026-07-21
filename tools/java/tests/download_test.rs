use proto_pdk_test_utils::*;
use std::path::PathBuf;

fn create_download_input(version: &str) -> DownloadPrebuiltInput {
    DownloadPrebuiltInput {
        context: PluginContext {
            version: VersionSpec::parse(version).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn create_locate_input(version: &str) -> LocateExecutablesInput {
    LocateExecutablesInput {
        context: PluginContext {
            version: VersionSpec::parse(version).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}

mod java_tool {
    use super::*;

    mod temurin {
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
                .download_prebuilt(create_download_input("temurin-21.0.11+10"))
                .await;

            assert_eq!(output.archive_prefix, Some("*".into()));
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
                .download_prebuilt(create_download_input("temurin-21.0.11+10"))
                .await;

            assert_eq!(output.archive_prefix, Some("*".into()));
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
                .download_prebuilt(create_download_input("temurin-21.0.11+10"))
                .await;

            assert_eq!(output.archive_prefix, Some("*".into()));
            assert_eq!(
                output.download_url,
                "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.11%2B10/OpenJDK21U-jdk_x64_windows_hotspot_21.0.11_10.zip"
            );
            assert_eq!(
                output.download_name,
                Some("OpenJDK21U-jdk_x64_windows_hotspot_21.0.11_10.zip".into())
            );
            assert!(output.checksum.is_some());
        }
    }

    mod openjdk {
        use super::*;

        // The jdk.java.net GA archives are frozen, so these URLs never change

        #[tokio::test(flavor = "multi_thread")]
        async fn supports_linux_x64() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("openjdk-21.0.2+13"))
                .await;

            assert_eq!(output.archive_prefix, Some("*".into()));
            assert_eq!(
                output.download_url,
                "https://download.java.net/java/GA/jdk21.0.2/f2283984656d49d69e91c558476027ac/13/GPL/openjdk-21.0.2_linux-x64_bin.tar.gz"
            );
            assert_eq!(
                output.download_name,
                Some("openjdk-21.0.2_linux-x64_bin.tar.gz".into())
            );
            // jdk.java.net does not provide checksums through foojay
            assert!(output.checksum.is_none());
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
                .download_prebuilt(create_download_input("openjdk-17.0.2+8"))
                .await;

            assert_eq!(output.archive_prefix, Some("*".into()));
            assert_eq!(
                output.download_url,
                "https://download.java.net/java/GA/jdk17.0.2/dfd4a8d0985749f896bed50d7138ee7f/8/GPL/openjdk-17.0.2_windows-x64_bin.zip"
            );
        }
    }

    mod vendors {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn zulu_prefers_tar_gz_from_cdn() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("zulu-21.0.11+10"))
                .await;

            let name = output.download_name.unwrap();

            // zulu also publishes zips for linux, so tar.gz must win
            assert!(name.starts_with("zulu"));
            assert!(name.contains("-ca-jdk21.0.11-linux_x64"));
            assert!(name.ends_with(".tar.gz"));
            assert!(
                output
                    .download_url
                    .starts_with("https://cdn.azul.com/zulu/bin/zulu")
            );
            assert!(output.checksum.is_some());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn liberica_supports_macos() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::MacOS, HostArch::Arm64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("liberica-21.0.11+11"))
                .await;

            assert_eq!(output.archive_prefix, Some("*".into()));
            assert_eq!(
                output.download_name,
                Some("bellsoft-jdk21.0.11+11-macos-aarch64.tar.gz".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn corretto_supports_linux() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("corretto-21.0.11"))
                .await;

            let name = output.download_name.unwrap();

            // The corretto revision (4th/5th components) may be respun
            assert!(name.starts_with("amazon-corretto-21.0.11."));
            assert!(name.ends_with("-linux-x64.tar.gz"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sap_machine_skips_musl_variant_on_gnu_host() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("sapmachine-21.0.11"))
                .await;

            // The API lists the musl tar.gz before the glibc one,
            // which must be filtered out for a gnu host
            assert_eq!(
                output.download_name,
                Some("sapmachine-jdk-21.0.11_linux-x64_bin.tar.gz".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn microsoft_supports_macos() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::MacOS, HostArch::Arm64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("microsoft-21.0.11"))
                .await;

            assert_eq!(output.archive_prefix, Some("*".into()));
            assert_eq!(
                output.download_name,
                Some("microsoft-jdk-21.0.11-macos-aarch64.tar.gz".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn openlogic_double_nests_on_macos() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::MacOS, HostArch::X64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("openlogic-25.0.3+9"))
                .await;

            // macOS archives nest as <stem>/jdk-x.x.x.jdk/Contents/Home
            assert_eq!(output.archive_prefix, Some("*/*".into()));
            assert_eq!(
                output.download_name,
                Some("openlogic-openjdk-25.0.3+9-mac-x64.zip".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn openlogic_single_nests_on_linux() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("openlogic-25.0.3+9"))
                .await;

            assert_eq!(output.archive_prefix, Some("*".into()));
        }
    }

    mod jre_package {
        use super::*;

        // Installing under the "jre" identifier downloads JRE packages

        #[tokio::test(flavor = "multi_thread")]
        async fn downloads_jre_archives() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("jre", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;

            let output = plugin
                .download_prebuilt(create_download_input("temurin-21.0.11+10"))
                .await;

            assert_eq!(
                output.download_name,
                Some("OpenJDK21U-jre_x64_linux_hotspot_21.0.11_10.tar.gz".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn only_locates_java() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("jre", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;

            let output = plugin
                .locate_executables(create_locate_input("temurin-21.0.11+10"))
                .await;

            assert!(output.exes.contains_key("java"));
            assert!(!output.exes.contains_key("javac"));
            assert!(!output.exes.contains_key("jar"));
        }
    }

    mod executables {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_linux_bins() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::Linux, HostArch::X64);
                })
                .await;

            let output = plugin
                .locate_executables(create_locate_input("temurin-21.0.11+10"))
                .await;

            assert_eq!(
                output.exes.get("java").unwrap().exe_path,
                Some("bin/java".into())
            );
            assert_eq!(
                output.exes.get("javac").unwrap().exe_path,
                Some("bin/javac".into())
            );
            assert_eq!(
                output.exes.get("jar").unwrap().exe_path,
                Some("bin/jar".into())
            );
            assert_eq!(output.exes_dirs, vec![PathBuf::from("bin")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_windows_bins() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::Windows, HostArch::X64);
                })
                .await;

            let output = plugin
                .locate_executables(create_locate_input("temurin-21.0.11+10"))
                .await;

            assert_eq!(
                output.exes.get("java").unwrap().exe_path,
                Some("bin/java.exe".into())
            );
            assert_eq!(output.exes_dirs, vec![PathBuf::from("bin")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_macos_bundle_bins() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::MacOS, HostArch::Arm64);
                })
                .await;

            let output = plugin
                .locate_executables(create_locate_input("temurin-21.0.11+10"))
                .await;

            assert_eq!(
                output.exes.get("java").unwrap().exe_path,
                Some("Contents/Home/bin/java".into())
            );
            assert_eq!(output.exes_dirs, vec![PathBuf::from("Contents/Home/bin")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn locates_macos_flat_bins_for_liberica() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("java-test", |config| {
                    config.host(HostOS::MacOS, HostArch::Arm64);
                })
                .await;

            let output = plugin
                .locate_executables(create_locate_input("liberica-21.0.11+11"))
                .await;

            // Liberica macOS archives are flat, with no Contents/Home bundle
            assert_eq!(
                output.exes.get("java").unwrap().exe_path,
                Some("bin/java".into())
            );
            assert_eq!(output.exes_dirs, vec![PathBuf::from("bin")]);
        }
    }
}
