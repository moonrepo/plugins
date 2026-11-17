use moon_common::Id;
use moon_config::DockerPruneConfig;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;
use starbase_utils::json::JsonValue;

mod javascript_toolchain_tier1 {
    use super::*;

    mod initialize_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_bun_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("bun.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("bun".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_bun_via_lockfile2() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("bun.lockb", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("bun".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_bun_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package.json", r#"{ "packageManager": "bun" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("bun".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_deno_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("deno.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("deno".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_npm_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package-lock.json", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("npm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_npm_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package.json", r#"{ "packageManager": "npm@10" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("npm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_nub_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("nub.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("nub".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_nub_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package.json", r#"{ "packageManager": "nub@0.7.0" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("nub".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_pnpm_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("pnpm-lock.yaml", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("pnpm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_pnpm_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package.json", r#"{ "packageManager": "pnpm@10.0.0" }"#);

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("pnpm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_yarn_via_lockfile() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("yarn.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("yarn".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_yarn_via_package_json() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{ "packageManager": "yarn@10.0.0+abcd" }"#,
            );

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("yarn".into())
            );
        }
    }

    mod detect_package_manager {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn detects_via_dev_engines() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{ "devEngines": { "packageManager": { "name": "pnpm", "version": "^11.0.0" } } }"#,
            );

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("pnpm".into())
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn skips_unknown_package_managers() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("package.json", r#"{ "packageManager": "unknown@1.0.0" }"#);
            sandbox.create_file("yarn.lock", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .initialize_toolchain(InitializeToolchainInput::default())
                .await;

            assert_eq!(
                output.default_settings.get("packageManager").unwrap(),
                &JsonValue::String("yarn".into())
            );
        }
    }

    mod scaffold_docker {
        use super::*;

        fn create_workspace() -> moon_pdk_test_utils::MoonWasmSandbox {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "package.json",
                r#"{ "workspaces": ["apps/*", "tools/*", "!tools/ignored"] }"#,
            );
            sandbox.create_file(
                "apps/a/package.json",
                json!({
                    "name": "a",
                    "dependencies": {
                        "lib": "file:../../vendor/lib",
                        "tgz": "file:../../vendor/tgz-1.0.0.tgz",
                        "outside": "file:../../../outside"
                    }
                })
                .to_string(),
            );
            sandbox.create_file("tools/x/package.json", r#"{ "name": "x" }"#);
            sandbox.create_file("tools/x/node_modules/y/package.json", r#"{ "name": "y" }"#);
            sandbox.create_file("tools/ignored/package.json", r#"{ "name": "ignored" }"#);
            sandbox.create_file("vendor/lib/package.json", r#"{ "name": "lib" }"#);
            sandbox.create_file("vendor/lib/index.js", "");
            sandbox.create_file("vendor/tgz-1.0.0.tgz", "");
            sandbox
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn copies_members_that_arent_projects() {
            let sandbox = create_workspace();
            let plugin = sandbox.create_toolchain("javascript").await;

            let mut output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::new("/workspace"),
                    output_dir: VirtualPath::new("/workspace/.moon/docker/configs"),
                    phase: ScaffoldDockerPhase::Configs,
                    toolchain_config: json!({ "packageManager": "npm" }),
                    ..Default::default()
                })
                .await;

            output.copied_files.sort_by_key(|path| path.to_string());

            assert_eq!(
                output.copied_files,
                [
                    VirtualPath::new("/workspace/.moon/docker/configs/apps/a/package.json"),
                    VirtualPath::new("/workspace/.moon/docker/configs/tools/x/package.json"),
                    VirtualPath::new("/workspace/.moon/docker/configs/vendor/lib/package.json"),
                    VirtualPath::new("/workspace/.moon/docker/configs/vendor/tgz-1.0.0.tgz"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn copies_a_projects_local_dependencies() {
            let sandbox = create_workspace();
            let plugin = sandbox.create_toolchain("javascript").await;

            let mut output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::new("/workspace/apps/a"),
                    output_dir: VirtualPath::new("/workspace/.moon/docker/configs/apps/a"),
                    phase: ScaffoldDockerPhase::Configs,
                    project: Some(ProjectFragment {
                        id: Id::raw("a"),
                        source: "apps/a".into(),
                        ..Default::default()
                    }),
                    toolchain_config: json!({ "packageManager": "npm" }),
                    ..Default::default()
                })
                .await;

            output.copied_files.sort_by_key(|path| path.to_string());

            assert_eq!(
                output.copied_files,
                [
                    VirtualPath::new("/workspace/.moon/docker/configs/vendor/lib/package.json"),
                    VirtualPath::new("/workspace/.moon/docker/configs/vendor/tgz-1.0.0.tgz"),
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_in_the_sources_phase() {
            let sandbox = create_workspace();
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::new("/workspace"),
                    output_dir: VirtualPath::new("/workspace/.moon/docker/sources"),
                    phase: ScaffoldDockerPhase::Sources,
                    toolchain_config: json!({ "packageManager": "npm" }),
                    ..Default::default()
                })
                .await;

            assert!(output.copied_files.is_empty());
        }
    }

    mod prune_docker {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_modules_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_disabled() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("node_modules/file", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: false,
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_modules_dir() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("node_modules/file", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert!(!sandbox.path().join("node_modules").exists());

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/node_modules")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_projects_modules_dirs() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("node_modules/file", "");
            sandbox.create_file("apps/a/node_modules/file", "");
            sandbox.create_file("libs/b/node_modules/file", "");
            sandbox.create_file("libs/c/node_modules/file", "");

            let plugin = sandbox.create_toolchain("javascript").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    projects: vec![ProjectFragment {
                        id: Id::raw("a"),
                        source: "apps/a".into(),
                        ..Default::default()
                    }],
                    project_dependencies: vec![ProjectFragment {
                        id: Id::raw("b"),
                        source: "libs/b".into(),
                        ..Default::default()
                    }],
                    root: VirtualPath::new(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert!(!sandbox.path().join("apps/a/node_modules").exists());
            assert!(!sandbox.path().join("libs/b/node_modules").exists());
            assert!(sandbox.path().join("libs/c/node_modules").exists());

            assert_eq!(
                output.changed_files,
                [
                    VirtualPath::new("/workspace/node_modules"),
                    VirtualPath::new("/workspace/apps/a/node_modules"),
                    VirtualPath::new("/workspace/libs/b/node_modules"),
                ]
            );
        }
    }
}
