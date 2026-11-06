use moon_config::DockerPruneConfig;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

mod go_toolchain_tier1 {
    use super::*;

    mod register_toolchain {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn has_no_vendor_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .register_toolchain(RegisterToolchainInput { id: Id::raw("go") })
                .await;

            assert!(output.vendor_dir_name.is_none());
        }
    }

    mod define_docker_metadata {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_image_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            for (version, image) in [
                (json!(null), "golang:latest"),
                (json!("latest"), "golang:latest"),
                (json!("1.69.0"), "golang:1.69.0"),
                (json!("1.24"), "golang:1.24"),
                (json!("~1.24"), "golang:1.24"),
                (json!("^1.22.0"), "golang:1.22.0"),
            ] {
                let output = plugin
                    .define_docker_metadata(DefineDockerMetadataInput {
                        toolchain_config: json!({
                            "version": version
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(output.default_image.unwrap(), image, "{version}");
            }
        }
    }

    mod scaffold_docker {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn copies_workspace_member_manifests() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.work", "go 1.24\n\nuse (\n\t./a\n\t./tools\n)\n");
            sandbox.create_file("a/go.mod", "module example.com/a\n");
            sandbox.create_file("tools/go.mod", "module example.com/tools\n");
            sandbox.create_file("tools/go.sum", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::new(sandbox.path()),
                    output_dir: VirtualPath::new(sandbox.path().join("out")),
                    phase: ScaffoldDockerPhase::Configs,
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.copied_files,
                [
                    VirtualPath::new("/workspace/out/a/go.mod"),
                    VirtualPath::new("/workspace/out/tools/go.mod"),
                    VirtualPath::new("/workspace/out/tools/go.sum"),
                ]
            );
            assert!(sandbox.path().join("out/tools/go.sum").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn copies_local_replace_manifests() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "project/go.mod",
                "module example.com/project\n\nrequire (\n\texample.com/lib v0.0.0\n\texample.com/outside v0.0.0\n)\n\nreplace example.com/lib => ../third_party/lib\n\nreplace example.com/outside => ../../outside\n",
            );
            sandbox.create_file("third_party/lib/go.mod", "module example.com/lib\n");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::new(sandbox.path().join("project")),
                    output_dir: VirtualPath::new(sandbox.path().join("out/project")),
                    phase: ScaffoldDockerPhase::Configs,
                    project: Some(ProjectFragment::default()),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.copied_files,
                [VirtualPath::new("/workspace/out/third_party/lib/go.mod")]
            );
            assert!(sandbox.path().join("out/third_party/lib/go.mod").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_in_sources_phase() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("go.work", "go 1.24\n\nuse ./tools\n");
            sandbox.create_file("tools/go.mod", "module example.com/tools\n");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::new(sandbox.path()),
                    output_dir: VirtualPath::new(sandbox.path().join("out")),
                    phase: ScaffoldDockerPhase::Sources,
                    ..Default::default()
                })
                .await;

            assert!(output.copied_files.is_empty());
        }
    }

    mod prune_docker {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_vendor_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

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
            sandbox.create_file("vendor/file", "");

            let plugin = sandbox.create_toolchain("go").await;

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

            assert!(sandbox.path().join("vendor/file").exists());

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_vendor_dir() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("vendor/file", "");

            let plugin = sandbox.create_toolchain("go").await;

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

            assert!(!sandbox.path().join("vendor").exists());

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/vendor")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_custom_vendor_dir() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("nested/.vendor/file", "");

            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    toolchain_config: json!({
                        "vendorDir": "nested/.vendor"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!sandbox.path().join("nested/.vendor").exists());

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/nested/.vendor")]
            );
        }
    }
}
