use moon_pdk_test_utils::{ExecuteExtensionInput, create_empty_moon_sandbox, create_moon_sandbox};

mod unpack_extension {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "the following required arguments were not provided")]
    async fn errors_if_no_args() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;
    }

    // #[tokio::test(flavor = "multi_thread")]
    // #[should_panic(
    //     expected = "Invalid source, only .tar, .tar.gz, and .zip archives are supported."
    // )]
    // async fn errors_if_unsupported_ext() {
    //     let sandbox = create_empty_moon_sandbox();
    //     let plugin = sandbox.create_extension("test").await;

    //     plugin
    //         .execute_extension(ExecuteExtensionInput {
    //             args: vec![
    //                 "--src".into(),
    //                 "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
    //             ],
    //             ..Default::default()
    //         })
    //         .await;
    // }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "must be a valid file")]
    async fn errors_if_src_file_missing() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec!["--src".into(), "./some/archive.zip".into()],
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "must be a directory, found a file")]
    async fn errors_if_dest_is_a_file() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        sandbox.create_file("dest", "file");

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--src".into(),
                    "https://github.com/moonrepo/moon/archive/refs/tags/v1.0.0.zip".into(),
                    "--dest".into(),
                    "./dest".into(),
                ],
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unpacks_tar() {
        let sandbox = create_moon_sandbox("tar");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--src".into(),
                    "./archive.tar".into(),
                    "--dest".into(),
                    "./out".into(),
                ],
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join("out/dir/file.txt").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unpacks_tar_gz() {
        let sandbox = create_moon_sandbox("tar");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--src".into(),
                    "./archive.tar.gz".into(),
                    "--dest".into(),
                    "./out".into(),
                ],
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join("out/dir/file.txt").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unpacks_zip() {
        let sandbox = create_moon_sandbox("zip");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--src".into(),
                    "./archive.zip".into(),
                    "--dest".into(),
                    "./out".into(),
                ],
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join("out/dir/file.txt").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_and_unpacks_tar() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--src".into(),
                    "https://github.com/moonrepo/moon/archive/refs/tags/v1.0.0.tar.gz".into(),
                    "--dest".into(),
                    "./out".into(),
                ],
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join(".moon/temp/v1.0.0.tar.gz").exists());
        assert!(sandbox.path().join("out/moon-1.0.0/README.md").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_and_unpacks_zip() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--src".into(),
                    "https://github.com/moonrepo/moon/archive/refs/tags/v1.0.0.zip".into(),
                    "--dest".into(),
                    "./out".into(),
                ],
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join(".moon/temp/v1.0.0.zip").exists());
        assert!(sandbox.path().join("out/moon-1.0.0/README.md").exists());
    }
}
