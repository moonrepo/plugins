use proto_pdk_test_utils::*;

mod dotnet_tool {
    use super::*;
    use ::dotnet_tool::DotnetToolConfig;

    // Files published for a released SDK never change, so 8.0.424 is a stable
    // fixture.
    const VERSION: &str = "8.0.424";

    async fn download(os: HostOS, arch: HostArch) -> DownloadPrebuiltOutput {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("dotnet-test", |config| {
                config.host(os, arch);
            })
            .await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse(VERSION).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn maps_every_supported_platform_to_a_rid() {
        let cases = [
            (HostOS::Windows, HostArch::X64, "win-x64.zip"),
            (HostOS::Windows, HostArch::X86, "win-x86.zip"),
            (HostOS::Windows, HostArch::Arm64, "win-arm64.zip"),
            (HostOS::MacOS, HostArch::X64, "osx-x64.tar.gz"),
            (HostOS::MacOS, HostArch::Arm64, "osx-arm64.tar.gz"),
            (HostOS::Linux, HostArch::X64, "linux-x64.tar.gz"),
            (HostOS::Linux, HostArch::Arm64, "linux-arm64.tar.gz"),
            (HostOS::Linux, HostArch::Arm, "linux-arm.tar.gz"),
        ];

        for (os, arch, suffix) in cases {
            let output = download(os, arch).await;

            assert!(
                output.download_url.ends_with(suffix),
                "{os:?}/{arch:?} produced {}",
                output.download_url
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn verifies_against_the_metadata_checksum() {
        let output = download(HostOS::Linux, HostArch::X64).await;
        let checksum = output.checksum.expect("no checksum was returned");

        // The release metadata carries a SHA512 per file, so a download needs
        // no separate checksum request.
        assert_eq!(checksum.algo, ChecksumAlgorithm::Sha512);
        assert_eq!(
            checksum.hash.as_ref().map(|hash| hash.len()),
            Some(128),
            "expected a 128 character SHA512"
        );
        assert!(output.checksum_url.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unpacks_without_stripping_a_prefix() {
        let output = download(HostOS::Linux, HostArch::X64).await;

        // The archive root *is* the SDK root: muxer, host/fxr, shared runtimes,
        // sdk band, packs and templates. There is no wrapping directory.
        assert_eq!(output.archive_prefix, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn selects_musl_archives_on_musl_hosts() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("dotnet-test", |config| {
                config.host_with(|host| {
                    host.os = HostOS::Linux;
                    host.arch = HostArch::X64;
                    host.libc = HostLibc::Musl;
                });
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse(VERSION).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        // The glibc archive will not run on Alpine.
        assert!(
            output.download_url.ends_with("linux-musl-x64.tar.gz"),
            "got {}",
            output.download_url
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dist_url_overrides_the_metadata_and_its_checksum() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("dotnet-test", |config| {
                config
                    .host(HostOS::Linux, HostArch::X64)
                    .tool_config(DotnetToolConfig {
                        dist_url: Some(
                            "https://mirror.internal/{version}/sdk-{rid}.{extension}".into(),
                        ),
                        ..Default::default()
                    });
            })
            .await;

        let output = plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse(VERSION).unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.download_url,
            "https://mirror.internal/8.0.424/sdk-linux-x64.tar.gz"
        );
        // A mirror's archives are not the ones the metadata has hashes for.
        assert!(output.checksum.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "8.0.999")]
    async fn errors_when_the_version_is_not_published() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        plugin
            .download_prebuilt(DownloadPrebuiltInput {
                context: PluginContext {
                    version: VersionSpec::parse("8.0.999").unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn never_symlinks_the_muxer() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("dotnet-test").await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;
        let exe = output.exes.get("dotnet").expect("no dotnet executable");

        // The muxer finds `host/fxr` and `shared/` relative to its own path on
        // disk, so it only works from inside its install directory. proto's
        // `bin` entries are symlinks, and a symlinked muxer fails outright.
        assert!(exe.no_bin);
        assert!(exe.primary);
    }
}
