use proto_pdk_test_utils::*;
use starbase_sandbox::locate_fixture;

async fn create_plugin(
    os: HostOS,
    arch: HostArch,
    libc: HostLibc,
) -> (ProtoWasmSandbox, WasmTestWrapper) {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox
        .create_schema_plugin_with_config(
            "schema-test",
            locate_fixture("schemas").join("v1-platform.toml"),
            |config| {
                config.host_with(|env| {
                    env.os = os;
                    env.arch = arch;
                    env.libc = libc;
                });
            },
        )
        .await;

    (sandbox, plugin)
}

fn context(version: &str) -> PluginContext {
    PluginContext {
        version: VersionSpec::parse(version).unwrap(),
        ..Default::default()
    }
}

async fn download(plugin: &WasmTestWrapper, version: &str) -> DownloadPrebuiltOutput {
    plugin
        .download_prebuilt(DownloadPrebuiltInput {
            context: context(version),
            ..Default::default()
        })
        .await
}

async fn try_download(plugin: &WasmTestWrapper, version: &str) -> Result<(), String> {
    plugin
        .tool
        .plugin
        .call_func_with::<_, _, DownloadPrebuiltOutput>(
            PluginFunction::DownloadPrebuilt,
            DownloadPrebuiltInput {
                context: context(version),
                ..Default::default()
            },
        )
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

async fn locate(plugin: &WasmTestWrapper, version: &str) -> LocateExecutablesOutput {
    plugin
        .locate_executables(LocateExecutablesInput {
            context: context(version),
            ..Default::default()
        })
        .await
}

mod v1_platform {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn interpolates_all_platform_fields() {
        let (_sandbox, plugin) = create_plugin(HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        assert_eq!(
            download(&plugin, "1.2.3").await,
            DownloadPrebuiltOutput {
                archive_prefix: Some("tool-1.2.3-amd64".into()),
                checksum_name: Some("tool-amd64.sha256".into()),
                checksum_url: Some("https://example.com/v1.2.3/tool-amd64.sha256".into()),
                download_name: Some("tool-linux-amd64-gnu.tar.gz".into()),
                download_url: "https://example.com/v1.2.3/tool-linux-amd64-gnu.tar.gz".into(),
                ..Default::default()
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn platform_arch_and_libc_beat_install_maps() {
        let (_sandbox, plugin) =
            create_plugin(HostOS::Linux, HostArch::Arm64, HostLibc::Musl).await;

        assert_eq!(
            download(&plugin, "1.2.3").await.download_name,
            Some("tool-linux-arm64-musl-static.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_for_arch_not_in_archs() {
        let (_sandbox, plugin) = create_plugin(HostOS::Linux, HostArch::X86, HostLibc::Gnu).await;

        assert!(try_download(&plugin, "1.2.3").await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bsd_falls_back_to_linux() {
        let (_sandbox, plugin) =
            create_plugin(HostOS::FreeBSD, HostArch::X64, HostLibc::Unknown).await;

        assert_eq!(
            download(&plugin, "1.2.3").await.download_name,
            Some("tool-freebsd-amd64-unknown.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_for_unsupported_os() {
        let (_sandbox, plugin) =
            create_plugin(HostOS::Solaris, HostArch::X64, HostLibc::Unknown).await;

        assert!(try_download(&plugin, "1.2.3").await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn canary_uses_canary_url() {
        let (_sandbox, plugin) = create_plugin(HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        let output = download(&plugin, "canary").await;

        assert_eq!(
            output.download_url,
            "https://example.com/canary/tool-linux-amd64-gnu.tar.gz"
        );
        assert_eq!(
            output.checksum_url,
            Some("https://example.com/vcanary/tool-amd64.sha256".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_with_platform_exe_path_and_dirs() {
        let (_sandbox, plugin) = create_plugin(HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        let output = locate(&plugin, "1.2.3").await;
        let primary = output.exes.get("schema-test").unwrap();
        let helper = output.exes.get("helper").unwrap();

        assert!(primary.primary);
        assert_eq!(primary.exe_path, Some("lin/1.2.3/tool".into()));
        assert_eq!(helper.exe_path, Some("helpers/1.2.3/helper".into()));
        // `exes-dirs` wins over deprecated `exes-dir` when both set
        assert_eq!(output.exes_dirs, vec![std::path::PathBuf::from("lin/new")]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_windows_with_exe_ext() {
        let (_sandbox, plugin) =
            create_plugin(HostOS::Windows, HostArch::X64, HostLibc::Unknown).await;

        let output = locate(&plugin, "1.2.3").await;

        assert_eq!(
            output.exes.get("schema-test").unwrap().exe_path,
            Some("win/tool.exe".into())
        );
        assert_eq!(
            output.exes.get("helper").unwrap().exe_path,
            Some("helpers/1.2.3/helper.exe".into())
        );
        assert!(output.exes_dirs.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_primary_from_id_without_platform_exe_path() {
        let (_sandbox, plugin) =
            create_plugin(HostOS::MacOS, HostArch::Arm64, HostLibc::Unknown).await;

        assert_eq!(
            locate(&plugin, "1.2.3")
                .await
                .exes
                .get("schema-test")
                .unwrap()
                .exe_path,
            Some("schema-test".into())
        );
    }
}

mod v1_versions {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn latest_alias_sets_latest() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin(
                "schema-test",
                locate_fixture("schemas").join("v1-versions.toml"),
            )
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;
        let latest = UnresolvedVersionSpec::parse("1.0.0").unwrap();

        assert_eq!(output.latest.as_ref(), Some(&latest));
        assert_eq!(output.aliases.get("latest"), Some(&latest));
    }
}
