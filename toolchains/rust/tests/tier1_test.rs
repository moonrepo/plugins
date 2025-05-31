use moon_config::DockerPruneConfig;
use moon_pdk_api::*;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

mod rust_toolchain_tier1 {
    use super::*;

    mod define_docker_metadata {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_image_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "rust:latest");

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({
                        "version": "1.69.0"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "rust:1.69.0");
        }
    }

    mod scaffold_docker {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_files_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;
            let output_dir = sandbox.path().join("out");

            assert!(!output_dir.join("src/lib.rs").exists());
            assert!(!output_dir.join("src/main.rs").exists());

            let output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::Real(sandbox.path().join("in")),
                    output_dir: VirtualPath::Real(output_dir.clone()),
                    phase: ScaffoldDockerPhase::Configs,
                    ..Default::default()
                })
                .await;

            assert!(output_dir.join("src/lib.rs").exists());
            assert!(output_dir.join("src/main.rs").exists());

            assert_eq!(
                output.copied_files,
                [
                    PathBuf::from("/workspace/out/src/lib.rs"),
                    PathBuf::from("/workspace/out/src/main.rs")
                ]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_files_in_sources_phase() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;
            let output_dir = sandbox.path().join("out");

            assert!(!output_dir.join("src/lib.rs").exists());
            assert!(!output_dir.join("src/main.rs").exists());

            let output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::Real(sandbox.path().join("in")),
                    output_dir: VirtualPath::Real(output_dir.clone()),
                    phase: ScaffoldDockerPhase::Sources,
                    ..Default::default()
                })
                .await;

            assert!(!output_dir.join("src/lib.rs").exists());
            assert!(!output_dir.join("src/main.rs").exists());

            assert!(output.copied_files.is_empty());
        }
    }

    mod prune_docker {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_no_target_dir() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    root: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_disabled() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("target/file", "");

            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: false,
                        ..Default::default()
                    },
                    root: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_target_dir_if_no_exes() {
            let sandbox = create_moon_sandbox("prune");
            let plugin = sandbox.create_toolchain("rust").await;

            fs::remove_dir_all(sandbox.path().join("target/release")).unwrap();

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    root: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert!(!sandbox.path().join("target").exists());
            assert!(!sandbox.path().join("target/other-file").exists());

            assert_eq!(output.changed_files, [PathBuf::from("/workspace/target")]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn preserves_target_dir_if_has_exes() {
            let sandbox = create_moon_sandbox("prune");
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    root: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert!(sandbox.path().join("target").exists());
            assert!(!sandbox.path().join("target/other-file").exists());
            assert!(
                sandbox
                    .path()
                    .join("target/release")
                    .join(if cfg!(windows) {
                        "rust_tc_test.exe"
                    } else {
                        "rust_tc_test"
                    })
                    .exists()
            );

            assert_eq!(output.changed_files, [PathBuf::from("/workspace/target")]);
        }
    }
}
