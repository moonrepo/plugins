use proto_pdk_test_utils::*;

mod swift_tool {
    use super::*;
    use ::swift_tool::{LinuxPlatform, SwiftToolConfig};

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("swift-test", |config| {
                config.host(HostOS::MacOS, HostArch::Arm64);
            })
            .await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("swift"),
            })
            .await;

        assert_eq!(metadata.name, "Swift");
        assert!(metadata.lock_options.metadata.is_empty());
        assert_eq!(metadata.minimum_proto_version, Some(Version::new(0, 61, 0)));
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_default_linux_platform_lock_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("swift-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("swift"),
            })
            .await;

        assert_eq!(
            metadata
                .lock_options
                .metadata
                .get("platform")
                .map(String::as_str),
            Some("ubuntu-24.04")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_linux_platform_lock_metadata() {
        let platforms = [
            (LinuxPlatform::AmazonLinux2, "amazon-linux-2"),
            (LinuxPlatform::AmazonLinux2023, "amazon-linux-2023"),
            (LinuxPlatform::Debian12, "debian-12"),
            (LinuxPlatform::Fedora39, "fedora-39"),
            (LinuxPlatform::Fedora41, "fedora-41"),
            (LinuxPlatform::RedhatUbi9, "redhat-ubi-9"),
            (LinuxPlatform::Ubuntu2004, "ubuntu-20.04"),
            (LinuxPlatform::Ubuntu2204, "ubuntu-22.04"),
            (LinuxPlatform::Ubuntu2404, "ubuntu-24.04"),
        ];

        for (linux_platform, expected) in platforms {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox
                .create_plugin_with_config("swift-test", |config| {
                    config
                        .host(HostOS::Linux, HostArch::X64)
                        .tool_config(SwiftToolConfig {
                            linux_platform: linux_platform.clone(),
                            ..Default::default()
                        });
                })
                .await;

            let metadata = plugin
                .register_tool(RegisterToolInput {
                    id: Id::raw("swift"),
                })
                .await;

            assert_eq!(
                metadata
                    .lock_options
                    .metadata
                    .get("platform")
                    .map(String::as_str),
                Some(expected)
            );
            assert_eq!(metadata.lock_options.metadata.len(), 1);
        }
    }
}
