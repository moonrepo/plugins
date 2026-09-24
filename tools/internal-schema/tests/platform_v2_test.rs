use proto_pdk_test_utils::*;
use starbase_sandbox::locate_fixture;
use std::collections::BTreeSet;
use std::path::PathBuf;

async fn create_plugin(
    fixture: &str,
    os: HostOS,
    arch: HostArch,
    libc: HostLibc,
) -> (ProtoWasmSandbox, WasmTestWrapper) {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox
        .create_schema_plugin_with_config(
            "schema-test",
            locate_fixture("schemas").join(fixture),
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

mod v2_platform {
    use super::*;

    const FIXTURE: &str = "v2-platform.toml";

    #[tokio::test(flavor = "multi_thread")]
    async fn interpolates_all_platform_fields() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

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
    async fn unmapped_arch_and_mapped_libc() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::Arm64, HostLibc::Musl).await;

        assert_eq!(
            download(&plugin, "1.2.3").await.download_name,
            Some("tool-linux-aarch64-musl-static.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_for_arch_not_in_archs() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X86, HostLibc::Gnu).await;

        assert!(try_download(&plugin, "1.2.3").await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bsd_falls_back_to_linux() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::FreeBSD, HostArch::X64, HostLibc::Unknown).await;

        assert_eq!(
            download(&plugin, "1.2.3").await.download_name,
            Some("tool-freebsd-amd64-unknown.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn errors_for_unsupported_os() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Solaris, HostArch::X64, HostLibc::Unknown).await;

        assert!(try_download(&plugin, "1.2.3").await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_exes_with_interpolation() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        let output = locate(&plugin, "1.2.3").await;

        assert_eq!(
            output.exes.get("tool").unwrap().exe_path,
            Some("bin/tool-1.2.3".into())
        );
        assert_eq!(output.exes_dirs, vec![PathBuf::from("bin")]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locate_uses_platform_exe_path_and_dirs() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Windows, HostArch::X64, HostLibc::Unknown).await;

        let output = locate(&plugin, "1.2.3").await;

        assert_eq!(
            output.exes.get("tool").unwrap().exe_path,
            Some("win/tool.exe".into())
        );
        assert_eq!(output.exes_dirs, vec![PathBuf::from("win/bin")]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_json_schema() {
        let (_sandbox, plugin) = create_plugin(
            "v2-platform.json",
            HostOS::Linux,
            HostArch::X64,
            HostLibc::Gnu,
        )
        .await;

        let output = download(&plugin, "1.2.3").await;

        assert_eq!(output.download_name, Some("tool-x86_64.tar.gz".into()));
        assert_eq!(
            output.download_url,
            "https://example.com/tool-x86_64.tar.gz"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn plugin_table_is_optional() {
        let (_sandbox, plugin) = create_plugin(
            "v2-no-plugin-table.toml",
            HostOS::Linux,
            HostArch::X64,
            HostLibc::Gnu,
        )
        .await;

        assert_eq!(
            download(&plugin, "1.2.3").await.download_name,
            Some("tool.tar.gz".into())
        );
    }
}

mod v2_overrides {
    use super::*;

    const FIXTURE: &str = "v2-overrides.toml";

    #[tokio::test(flavor = "multi_thread")]
    async fn base_when_no_range_matches() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        let output = download(&plugin, "1.5.0").await;

        assert_eq!(output.download_name, Some("tool-amd64.tar.gz".into()));
        assert_eq!(
            output.download_url,
            "https://example.com/v1.5.0/tool-amd64.tar.gz"
        );
        assert_eq!(output.archive_prefix, Some("tool-1.5.0".into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn base_archs_allow_arm64_before_override() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::Arm64, HostLibc::Gnu).await;

        assert_eq!(
            download(&plugin, "1.5.0").await.download_name,
            Some("tool-arm64.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn platform_override_replaces_file_and_merges_arch() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        let output = download(&plugin, "2.1.0").await;

        assert_eq!(output.download_name, Some("tool-v2-x86_64.tar.gz".into()));
        assert_eq!(
            output.download_url,
            "https://example.com/v2.1.0/tool-v2-x86_64.tar.gz"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn platform_override_replaces_archs() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::Arm64, HostLibc::Gnu).await;

        assert!(try_download(&plugin, "2.1.0").await.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn platform_override_for_other_os_is_ignored() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::MacOS, HostArch::Arm64, HostLibc::Unknown).await;

        assert_eq!(
            download(&plugin, "2.1.0").await.download_name,
            Some("tool-mac-aarch64.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn multiple_matching_ranges_stack() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        let output = download(&plugin, "3.0.0").await;

        // ^3 is declared later, so its install.download_name beats the platform
        // from >=2.0.0, while the arch map from >=2.0.0 still applies
        assert_eq!(output.download_name, Some("tool-v3-x86_64.tar.gz".into()));
        assert_eq!(
            output.download_url,
            "https://new.example.com/3.0.0/tool-v3-x86_64.tar.gz"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn install_override_beats_base_platform() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::MacOS, HostArch::Arm64, HostLibc::Unknown).await;

        // ^3 sets install.download_name, but macos only has a base platform entry
        assert_eq!(
            download(&plugin, "3.0.0").await.download_name,
            Some("tool-v3-aarch64.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bsd_with_explicit_entry_ignores_linux_override() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::FreeBSD, HostArch::X64, HostLibc::Unknown).await;

        assert_eq!(
            download(&plugin, "2.1.0").await.download_name,
            Some("tool-freebsd-x86_64.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bsd_without_entry_uses_linux_override() {
        let (_sandbox, plugin) = create_plugin(
            "v2-override-order.toml",
            HostOS::FreeBSD,
            HostArch::X64,
            HostLibc::Unknown,
        )
        .await;

        // Only linux is defined, and only >=1.0.0 matches
        assert_eq!(
            download(&plugin, "1.0.5").await.download_name,
            Some("a".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bsd_override_beats_linux_override() {
        let (_sandbox, plugin) = create_plugin(
            "v2-override-order.toml",
            HostOS::FreeBSD,
            HostArch::X64,
            HostLibc::Unknown,
        )
        .await;

        assert_eq!(
            download(&plugin, "1.5.0").await.download_name,
            Some("d-freebsd".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn prerelease_does_not_match_plain_range() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        assert_eq!(
            download(&plugin, "2.1.0-rc.1").await.download_name,
            Some("tool-amd64.tar.gz".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn canary_only_uses_canary_override() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        let output = download(&plugin, "canary").await;

        // The arch map from >=2.0.0 doesn't apply, so x64 is still amd64
        assert_eq!(
            output.download_name,
            Some("tool-nightly-amd64.tar.gz".into())
        );
        assert_eq!(
            output.download_url,
            "https://example.com/nightly/tool-nightly-amd64.tar.gz"
        );
        assert_eq!(output.archive_prefix, Some("tool-canary".into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locate_override_replaces_exes() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        assert_eq!(
            locate(&plugin, "1.0.0")
                .await
                .exes
                .get("tool")
                .unwrap()
                .exe_path,
            Some("bin/tool".into())
        );
        assert_eq!(
            locate(&plugin, "3.0.0")
                .await
                .exes
                .get("tool")
                .unwrap()
                .exe_path,
            Some("tool3/bin/tool".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn platform_beats_locate_in_same_override() {
        let (_sandbox, plugin) =
            create_plugin(FIXTURE, HostOS::MacOS, HostArch::Arm64, HostLibc::Unknown).await;

        assert_eq!(
            locate(&plugin, "3.0.0")
                .await
                .exes
                .get("tool")
                .unwrap()
                .exe_path,
            Some("tool3/mac/tool".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn overlapping_ranges_resolve_deterministically() {
        let mut seen = BTreeSet::new();

        for _ in 0..5 {
            let (_sandbox, plugin) = create_plugin(
                "v2-override-order.toml",
                HostOS::Linux,
                HostArch::X64,
                HostLibc::Gnu,
            )
            .await;

            for _ in 0..5 {
                seen.insert(download(&plugin, "1.5.0").await.download_name.unwrap());
            }
        }

        assert_eq!(seen.len(), 1, "got different results: {seen:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn last_declared_range_wins_toml() {
        let (_sandbox, plugin) = create_plugin(
            "v2-override-declared-order.toml",
            HostOS::Linux,
            HostArch::X64,
            HostLibc::Gnu,
        )
        .await;

        assert_eq!(
            download(&plugin, "1.6.0").await.download_name,
            Some("gte".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn last_declared_range_wins_json() {
        let (_sandbox, plugin) = create_plugin(
            "v2-override-declared-order.json",
            HostOS::Linux,
            HostArch::X64,
            HostLibc::Gnu,
        )
        .await;

        assert_eq!(
            download(&plugin, "1.6.0").await.download_name,
            Some("gte".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn last_declared_range_wins_yaml() {
        let (_sandbox, plugin) = create_plugin(
            "v2-override-declared-order.yaml",
            HostOS::Linux,
            HostArch::X64,
            HostLibc::Gnu,
        )
        .await;

        assert_eq!(
            download(&plugin, "1.6.0").await.download_name,
            Some("gte".into())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn partial_install_override() {
        let (_sandbox, plugin) = create_plugin(
            "v2-partial-install-override.toml",
            HostOS::Linux,
            HostArch::X64,
            HostLibc::Gnu,
        )
        .await;

        assert_eq!(
            download(&plugin, "2.0.0").await.checksum_url,
            Some("https://example.com/".into())
        );
    }
}

mod v2_versions {
    use super::*;

    async fn load(fixture: &str) -> LoadVersionsOutput {
        let (_sandbox, plugin) =
            create_plugin(fixture, HostOS::Linux, HostArch::X64, HostLibc::Gnu).await;

        plugin.load_versions(LoadVersionsInput::default()).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn latest_defaults_to_highest_stable_version() {
        let output = load("v2-versions.toml").await;
        let latest = UnresolvedVersionSpec::parse("2.0.0").unwrap();

        assert_eq!(output.latest.as_ref(), Some(&latest));
        assert_eq!(output.aliases.get("latest"), Some(&latest));
        assert_eq!(
            output.aliases.get("stable"),
            Some(&UnresolvedVersionSpec::parse("1.0.0").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn latest_can_be_configured() {
        let output = load("v2-versions-latest.toml").await;
        let latest = UnresolvedVersionSpec::parse("1.0.0").unwrap();

        assert_eq!(output.latest.as_ref(), Some(&latest));
        assert_eq!(output.aliases.get("latest"), Some(&latest));
    }
}

mod v2_resolve {
    use super::*;

    async fn has_pre_v1(plugin: &WasmTestWrapper, initial: &str) -> bool {
        plugin
            .load_versions(LoadVersionsInput {
                initial: UnresolvedVersionSpec::parse(initial).unwrap(),
                ..Default::default()
            })
            .await
            .versions
            .iter()
            .any(|version| {
                version
                    .as_version()
                    .is_some_and(|version| version.major == 0)
            })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn override_resolves_requests_within_range() {
        let (_sandbox, plugin) = create_plugin(
            "v2-resolve.toml",
            HostOS::Linux,
            HostArch::X64,
            HostLibc::Gnu,
        )
        .await;

        for initial in ["0.21", "0.21.3", ">=0.5 <2", "^0.1 || ^3"] {
            assert!(has_pre_v1(&plugin, initial).await, "{initial}");
        }

        for initial in ["latest", "^1", "1.2.3 - 2.0.0", "canary"] {
            assert!(!has_pre_v1(&plugin, initial).await, "{initial}");
        }
    }
}
