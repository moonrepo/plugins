use moon_config::DockerPruneConfig;
use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;
use std::path::PathBuf;

mod go_toolchain_tier1 {
    use super::*;

    mod define_docker_metadata {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_image_version() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("go").await;

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({}),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "golang:latest");

            let output = plugin
                .define_docker_metadata(DefineDockerMetadataInput {
                    toolchain_config: json!({
                        "version": "1.69.0"
                    }),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.default_image.unwrap(), "golang:1.69.0");
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
                    root: VirtualPath::Real(sandbox.path().into()),
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
                    root: VirtualPath::Real(sandbox.path().into()),
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
                    root: VirtualPath::Real(sandbox.path().into()),
                    ..Default::default()
                })
                .await;

            assert!(!sandbox.path().join("vendor").exists());

            assert_eq!(output.changed_files, [PathBuf::from("/workspace/vendor")]);
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
                    root: VirtualPath::Real(sandbox.path().into()),
                    toolchain_config: json!({
                        "vendorDir": "nested/.vendor"
                    }),
                    ..Default::default()
                })
                .await;

            assert!(!sandbox.path().join("nested/.vendor").exists());

            assert_eq!(
                output.changed_files,
                [PathBuf::from("/workspace/nested/.vendor")]
            );
        }
    }
}
