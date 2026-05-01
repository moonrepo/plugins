use proto_pdk_test_utils::*;
use std::path::PathBuf;

fn locate_input(sandbox: &ProtoWasmSandbox) -> LocateExecutablesInput {
    LocateExecutablesInput {
        install_dir: VirtualPath::Real(sandbox.path().into()),
        ..Default::default()
    }
}

mod cargo_backend_locate {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_files_in_bin_dir() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("bin/cargo-nextest", "");

        let plugin = sandbox.create_plugin("cargo:cargo-nextest").await;
        let output = plugin.locate_executables(locate_input(&sandbox)).await;

        let exe = output.exes.get("cargo-nextest").unwrap();
        assert_eq!(exe.exe_path, Some(PathBuf::from("bin/cargo-nextest")));
        assert!(exe.primary);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn skips_directories_in_bin() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("bin/cargo-nextest", "");
        // A subdirectory with a file inside, to ensure the directory itself is
        // skipped but the file inside isn't accidentally walked.
        sandbox.create_file("bin/subdir/inner", "");

        let plugin = sandbox.create_plugin("cargo:cargo-nextest").await;
        let output = plugin.locate_executables(locate_input(&sandbox)).await;

        assert!(output.exes.contains_key("cargo-nextest"));
        assert!(!output.exes.contains_key("subdir"));
        assert!(!output.exes.contains_key("inner"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn primary_for_id_match() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("bin/eza", "");
        sandbox.create_file("bin/other", "");

        let plugin = sandbox.create_plugin("cargo:eza").await;
        let output = plugin.locate_executables(locate_input(&sandbox)).await;

        let eza = output.exes.get("eza").unwrap();
        assert!(eza.primary);

        let other = output.exes.get("other").unwrap();
        assert!(!other.primary);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn primary_for_cargo_prefix_suffix() {
        // For `cargo-X` packages, the actual binary is sometimes named `X`
        // (e.g. `cargo-outdated` shipping an `outdated` binary).
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("bin/outdated", "");

        let plugin = sandbox.create_plugin("cargo:cargo-outdated").await;
        let output = plugin.locate_executables(locate_input(&sandbox)).await;

        let exe = output.exes.get("outdated").unwrap();
        assert!(exe.primary);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn falls_back_to_first_exe_as_primary() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("bin/something-else", "");

        let plugin = sandbox.create_plugin("cargo:cargo-nextest").await;
        let output = plugin.locate_executables(locate_input(&sandbox)).await;

        assert_eq!(output.exes.values().filter(|cfg| cfg.primary).count(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn strips_exe_suffix_from_name() {
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("bin/cargo-nextest.exe", "");

        let plugin = sandbox.create_plugin("cargo:cargo-nextest").await;
        let output = plugin.locate_executables(locate_input(&sandbox)).await;

        // Map key has `.exe` stripped, but exe_path retains it.
        let exe = output.exes.get("cargo-nextest").unwrap();
        assert_eq!(exe.exe_path, Some(PathBuf::from("bin/cargo-nextest.exe")));
        assert!(exe.primary);
    }
}
