use moon_config::DockerPruneConfig;
use moon_pdk_api::*;
use moon_pdk_test_utils::{MoonWasmSandbox, create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

        #[tokio::test(flavor = "multi_thread")]
        async fn handles_image_channels() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("rust").await;

            for (version, image) in [
                ("stable", "rust:latest"),
                ("beta", "rust:latest"),
                ("nightly", "rustlang/rust:nightly"),
                ("nightly-2024-01-01", "rustlang/rust:nightly"),
                ("1.69", "rust:1.69"),
            ] {
                let output = plugin
                    .define_docker_metadata(DefineDockerMetadataInput {
                        toolchain_config: json!({
                            "version": version
                        }),
                        ..Default::default()
                    })
                    .await;

                assert_eq!(output.default_image.unwrap(), image);
            }
        }
    }

    mod scaffold_docker {
        use super::*;

        const PACKAGE: &str = "[package]\nname = \"in\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

        async fn scaffold_configs(sandbox: &MoonWasmSandbox) -> Vec<VirtualPath> {
            let plugin = sandbox.create_toolchain("rust").await;

            let output = plugin
                .scaffold_docker(ScaffoldDockerInput {
                    input_dir: VirtualPath::new(sandbox.path().join("in")),
                    output_dir: VirtualPath::new(sandbox.path().join("out")),
                    phase: ScaffoldDockerPhase::Configs,
                    project: Some(ProjectFragment::default()),
                    ..Default::default()
                })
                .await;

            for file in &output.copied_files {
                let path = file.strip_prefix("/workspace").unwrap();

                assert!(sandbox.path().join(path).exists());
            }

            output.copied_files
        }

        fn stubs(paths: &[&str]) -> Vec<VirtualPath> {
            paths
                .iter()
                .map(|path| VirtualPath::new(format!("/workspace/out/{path}")))
                .collect()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_lib_file_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("in/Cargo.toml", PACKAGE);
            sandbox.create_file("in/src/lib.rs", "pub fn lib() {}");

            assert_eq!(scaffold_configs(&sandbox).await, stubs(&["src/lib.rs"]));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_main_file_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("in/Cargo.toml", PACKAGE);
            sandbox.create_file("in/src/main.rs", "fn main() {}");

            assert_eq!(scaffold_configs(&sandbox).await, stubs(&["src/main.rs"]));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_lib_and_main_files_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("in/Cargo.toml", PACKAGE);
            sandbox.create_file("in/src/lib.rs", "pub fn lib() {}");
            sandbox.create_file("in/src/main.rs", "fn main() {}");

            assert_eq!(
                scaffold_configs(&sandbox).await,
                stubs(&["src/lib.rs", "src/main.rs"])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_bin_files_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file("in/Cargo.toml", PACKAGE);
            sandbox.create_file("in/src/bin/a.rs", "fn main() {}");
            sandbox.create_file("in/src/bin/b/main.rs", "fn main() {}");

            assert_eq!(
                scaffold_configs(&sandbox).await,
                stubs(&["src/bin/a.rs", "src/bin/b/main.rs"])
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_named_bin_files_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "in/Cargo.toml",
                format!("{PACKAGE}\n[[bin]]\nname = \"a\"\n"),
            );
            sandbox.create_file("in/src/bin/a.rs", "fn main() {}");

            assert_eq!(scaffold_configs(&sandbox).await, stubs(&["src/bin/a.rs"]));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_explicit_path_files_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "in/Cargo.toml",
                format!(
                    "{PACKAGE}\n[lib]\npath = \"lib.rs\"\n\n[[bin]]\nname = \"a\"\npath = \"bin/a.rs\"\n"
                ),
            );
            sandbox.create_file("in/lib.rs", "pub fn lib() {}");
            sandbox.create_file("in/bin/a.rs", "fn main() {}");

            assert!(scaffold_configs(&sandbox).await.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_workspace_package_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "Cargo.toml",
                "[workspace]\nmembers = [\"in\"]\n\n[workspace.package]\nedition = \"2021\"\n",
            );
            sandbox.create_file(
                "in/Cargo.toml",
                "[package]\nname = \"in\"\nversion = \"0.1.0\"\nedition.workspace = true\n",
            );
            sandbox.create_file("in/src/bin/a.rs", "fn main() {}");

            assert_eq!(scaffold_configs(&sandbox).await, stubs(&["src/bin/a.rs"]));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn falls_back_to_lib_file_without_manifest_in_config_phase() {
            let sandbox = create_empty_moon_sandbox();

            assert_eq!(scaffold_configs(&sandbox).await, stubs(&["src/lib.rs"]));
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
                    input_dir: VirtualPath::new(sandbox.path().join("in")),
                    output_dir: VirtualPath::new(output_dir.clone()),
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
                    root: VirtualPath::new(sandbox.path()),
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
                    root: VirtualPath::new(sandbox.path()),
                    ..Default::default()
                })
                .await;

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn removes_target_dir_if_no_exes() {
            let sandbox = create_moon_sandbox("prune");

            fs::remove_dir_all(sandbox.path().join("target/release")).unwrap();

            let output = prune(&sandbox, None).await;

            assert!(!sandbox.path().join("target").exists());
            assert!(!sandbox.path().join("target/other-file").exists());

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/target")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn preserves_target_dir_if_has_exes() {
            let sandbox = create_moon_sandbox("prune");

            let output = prune(&sandbox, None).await;

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

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/target")]
            );
        }

        // Cargo reports the workspace root from the current directory, which
        // resolves symlinks on macOS (`/var` -> `/private/var`), while
        // Windows `canonicalize` adds a `\\?\` prefix that Cargo doesn't use
        fn real_path(path: &Path) -> PathBuf {
            if cfg!(windows) {
                path.to_path_buf()
            } else {
                path.canonicalize().unwrap()
            }
        }

        fn create_bin(sandbox: &MoonWasmSandbox, dir: &str) {
            sandbox.create_file(format!("{dir}/rust_tc_test"), "");
            sandbox.create_file(format!("{dir}/rust_tc_test.exe"), "");
        }

        fn has_bin(sandbox: &MoonWasmSandbox, dir: &str) -> bool {
            sandbox
                .path()
                .join(dir)
                .join(if cfg!(windows) {
                    "rust_tc_test.exe"
                } else {
                    "rust_tc_test"
                })
                .exists()
        }

        // The host may relocate the target directory for all projects
        async fn prune(
            sandbox: &MoonWasmSandbox,
            target_dir: Option<PathBuf>,
        ) -> PruneDockerOutput {
            unsafe {
                match target_dir {
                    Some(dir) => env::set_var("CARGO_TARGET_DIR", dir),
                    None => env::remove_var("CARGO_TARGET_DIR"),
                };
            }

            let plugin = sandbox.create_toolchain("rust").await;

            plugin
                .prune_docker(PruneDockerInput {
                    docker_config: DockerPruneConfig {
                        delete_vendor_directories: true,
                        ..Default::default()
                    },
                    root: VirtualPath::new(sandbox.path()),
                    ..Default::default()
                })
                .await
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn preserves_exes_in_target_triple_and_custom_profile_dirs() {
            let sandbox = create_moon_sandbox("prune");
            create_bin(&sandbox, "target/x86_64-unknown-linux-musl/release");
            create_bin(&sandbox, "target/dist");
            sandbox.create_file("target/release/deps/rust_tc_test-abc123", "");

            let output = prune(&sandbox, None).await;

            assert!(has_bin(&sandbox, "target/release"));
            assert!(has_bin(
                &sandbox,
                "target/x86_64-unknown-linux-musl/release"
            ));
            assert!(has_bin(&sandbox, "target/dist"));
            assert!(!sandbox.path().join("target/other-file").exists());
            assert!(!sandbox.path().join("target/release/deps").exists());

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/target")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_relocated_target_dir() {
            let sandbox = create_moon_sandbox("prune");
            create_bin(&sandbox, "build/cargo/release");
            sandbox.create_file("build/cargo/other-file", "");

            let output = prune(
                &sandbox,
                Some(real_path(sandbox.path()).join("build/cargo")),
            )
            .await;

            assert!(has_bin(&sandbox, "build/cargo/release"));
            assert!(!sandbox.path().join("build/cargo/other-file").exists());
            assert!(sandbox.path().join("target/other-file").exists());

            assert_eq!(
                output.changed_files,
                [VirtualPath::new("/workspace/build/cargo")]
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_target_dir_outside_root() {
            let sandbox = create_moon_sandbox("prune");
            let outside = create_empty_moon_sandbox();

            let output = prune(&sandbox, Some(real_path(outside.path()).join("target"))).await;

            assert!(sandbox.path().join("target/other-file").exists());
            assert!(output.changed_files.is_empty());
        }
    }
}
