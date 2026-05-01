use npm_backend::NpmBackendConfig;
use proto_pdk_test_utils::*;
use std::path::PathBuf;

fn locate_input(sandbox: &ProtoWasmSandbox) -> LocateExecutablesInput {
    LocateExecutablesInput {
        install_dir: VirtualPath::Real(sandbox.path().into()),
        ..Default::default()
    }
}

mod npm_backend_locate {
    use super::*;

    mod with_node {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn reads_object_bins_from_package_json() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "node_modules/typescript/package.json",
                r#"{"bin":{"tsc":"./bin/tsc","tsserver":"./bin/tsserver"}}"#,
            );

            let plugin = sandbox.create_plugin("npm:typescript").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            let tsc = output.exes.get("tsc").unwrap();
            assert_eq!(
                tsc.exe_path,
                Some(PathBuf::from("node_modules/typescript/bin/tsc"))
            );
            assert!(tsc.primary);
            assert_eq!(tsc.parent_exe_name, None);

            let tsserver = output.exes.get("tsserver").unwrap();
            assert_eq!(
                tsserver.exe_path,
                Some(PathBuf::from("node_modules/typescript/bin/tsserver"))
            );
            assert!(!tsserver.primary);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reads_string_bin_from_package_json() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "node_modules/prettier/package.json",
                r#"{"bin":"./bin/prettier.cjs"}"#,
            );

            let plugin = sandbox.create_plugin("npm:prettier").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            let prettier = output.exes.get("prettier").unwrap();
            assert_eq!(
                prettier.exe_path,
                Some(PathBuf::from("node_modules/prettier/bin/prettier.cjs"))
            );
            assert!(prettier.primary);
            assert_eq!(prettier.parent_exe_name, Some("node".into()));
            assert!(prettier.no_bin);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reads_scoped_package_using_unscoped_bin_name() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "node_modules/@moonrepo/cli/package.json",
                r#"{"bin":"./moon"}"#,
            );

            let plugin = sandbox.create_plugin("npm:@moonrepo/cli").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            assert!(output.exes.contains_key("cli"));
            assert!(!output.exes.contains_key("@moonrepo/cli"));

            let cli = output.exes.get("cli").unwrap();
            assert_eq!(
                cli.exe_path,
                Some(PathBuf::from("node_modules/@moonrepo/cli/moon"))
            );
            assert!(cli.primary);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_node_parent_for_js_extensions() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "node_modules/multi/package.json",
                r#"{"bin":{"a":"./a.js","b":"./b.cjs","c":"./c.mjs"}}"#,
            );

            let plugin = sandbox.create_plugin("npm:multi").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            for name in ["a", "b", "c"] {
                let cfg = output.exes.get(name).unwrap();
                assert_eq!(cfg.parent_exe_name, Some("node".into()));
                assert!(cfg.no_bin);
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn sets_tsx_parent_for_ts_extensions() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "node_modules/multi/package.json",
                r#"{"bin":{"a":"./a.ts","b":"./b.cts","c":"./c.mts","d":"./d.tsx"}}"#,
            );

            let plugin = sandbox.create_plugin("npm:multi").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            for name in ["a", "b", "c", "d"] {
                let cfg = output.exes.get(name).unwrap();
                assert_eq!(cfg.parent_exe_name, Some("tsx".into()));
                assert!(cfg.no_bin);
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn no_parent_for_extensionless_bin() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "node_modules/typescript/package.json",
                r#"{"bin":{"tsc":"./bin/tsc"}}"#,
            );

            let plugin = sandbox.create_plugin("npm:typescript").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            let tsc = output.exes.get("tsc").unwrap();
            assert_eq!(tsc.parent_exe_name, None);
            assert!(!tsc.no_bin);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn falls_back_to_bin_dir_when_no_package_json() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("bin/typescript", "");
            sandbox.create_file("bin/extra", "");

            let plugin = sandbox.create_plugin("npm:typescript").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            let typescript = output.exes.get("typescript").unwrap();
            assert_eq!(typescript.exe_path, Some(PathBuf::from("bin/typescript")));
            assert!(typescript.primary);

            let extra = output.exes.get("extra").unwrap();
            assert_eq!(extra.exe_path, Some(PathBuf::from("bin/extra")));
            assert!(!extra.primary);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn falls_back_to_install_dir_when_no_bin_dir() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("typescript", "");

            let plugin = sandbox.create_plugin("npm:typescript").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            let typescript = output.exes.get("typescript").unwrap();
            assert_eq!(typescript.exe_path, Some(PathBuf::from("typescript")));
            assert!(typescript.primary);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn fallback_skips_files_with_extensions() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("typescript", "");
            sandbox.create_file("typescript.cmd", "");
            sandbox.create_file("typescript.ps1", "");

            let plugin = sandbox.create_plugin("npm:typescript").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            assert!(output.exes.contains_key("typescript"));
            assert!(!output.exes.contains_key("typescript.cmd"));
            assert!(!output.exes.contains_key("typescript.ps1"));
            assert_eq!(output.exes.len(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn marks_first_exe_as_primary_when_none_match() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file("bin/foo", "");
            sandbox.create_file("bin/bar", "");

            let plugin = sandbox.create_plugin("npm:typescript").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            assert_eq!(output.exes.values().filter(|cfg| cfg.primary).count(), 1);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_bin_in_exes_dirs_when_present() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "node_modules/typescript/package.json",
                r#"{"bin":{"tsc":"./bin/tsc"}}"#,
            );
            sandbox.create_file("bin/.keep", "");

            let plugin = sandbox.create_plugin("npm:typescript").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            assert!(output.exes_dirs.contains(&PathBuf::from("bin")));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_root_in_exes_dirs_when_no_bin() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "node_modules/typescript/package.json",
                r#"{"bin":{"tsc":"./tsc"}}"#,
            );

            let plugin = sandbox.create_plugin("npm:typescript").await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            assert!(output.exes_dirs.contains(&PathBuf::from(".")));
        }
    }

    mod with_bun {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn reads_from_lib_node_modules() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "lib/node_modules/typescript/package.json",
                r#"{"bin":{"tsc":"./bin/tsc"}}"#,
            );

            let plugin = sandbox
                .create_plugin_with_config("npm:typescript", |cfg| {
                    cfg.backend_config(NpmBackendConfig { bun: true });
                })
                .await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            let tsc = output.exes.get("tsc").unwrap();
            assert_eq!(
                tsc.exe_path,
                Some(PathBuf::from("lib/node_modules/typescript/bin/tsc"))
            );
            assert!(tsc.primary);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn reads_scoped_package_from_lib_node_modules() {
            let sandbox = create_empty_proto_sandbox();
            sandbox.create_file(
                "lib/node_modules/@moonrepo/cli/package.json",
                r#"{"bin":"./moon"}"#,
            );

            let plugin = sandbox
                .create_plugin_with_config("npm:@moonrepo/cli", |cfg| {
                    cfg.backend_config(NpmBackendConfig { bun: true });
                })
                .await;
            let output = plugin.locate_executables(locate_input(&sandbox)).await;

            let cli = output.exes.get("cli").unwrap();
            assert_eq!(
                cli.exe_path,
                Some(PathBuf::from("lib/node_modules/@moonrepo/cli/moon"))
            );
            assert!(cli.primary);
        }
    }
}
