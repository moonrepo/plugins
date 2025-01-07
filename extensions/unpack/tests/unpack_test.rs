use moon_pdk_test_utils::{create_extension, ExecuteExtensionInput};
use starbase_sandbox::create_empty_sandbox;

mod unpack_extension {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "the following required arguments were not provided")]
    async fn errors_if_no_args() {
        let sandbox = create_empty_sandbox();
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(
        expected = "Invalid source, only .tar, .tar.gz, and .zip archives are supported."
    )]
    async fn errors_if_unsupported_ext() {
        let sandbox = create_empty_sandbox();
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--src".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                ],
                context: plugin.create_context(sandbox.path()),
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "must be a valid file")]
    async fn errors_if_src_file_missing() {
        let sandbox = create_empty_sandbox();
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec!["--src".into(), "./some/archive.zip".into()],
                context: plugin.create_context(sandbox.path()),
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "must be a directory, found a file")]
    async fn errors_if_dest_is_a_file() {
        let sandbox = create_empty_sandbox();
        let plugin = create_extension("test", sandbox.path());

        sandbox.create_file("dest", "file");

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--src".into(),
                    "https://github.com/moonrepo/moon/archive/refs/tags/v1.0.0.zip".into(),
                    "--dest".into(),
                    "./dest".into(),
                ],
                context: plugin.create_context(sandbox.path()),
            })
            .await;
    }

    // #[test]
    // fn unpacks_tar() {
    //     let sandbox = create_sandbox("tar");
    //     let plugin = create_extension("test", sandbox.path());

    //     plugin.execute_extension(ExecuteExtensionInput {
    //         args: vec![
    //             "--src".into(),
    //             "./archive.tar".into(),
    //             "--dest".into(),
    //             "./out".into(),
    //         ],
    //         context: plugin.create_context(sandbox.path()),
    //     });

    //     assert!(sandbox.path().join("out/file.txt").exists());
    // }

    // #[test]
    // fn unpacks_tar_gz() {
    //     let sandbox = create_sandbox("tar");
    //     let plugin = create_extension("test", sandbox.path());

    //     plugin.execute_extension(ExecuteExtensionInput {
    //         args: vec![
    //             "--src".into(),
    //             "./archive.tar.gz".into(),
    //             "--dest".into(),
    //             "./out".into(),
    //         ],
    //         context: plugin.create_context(sandbox.path()),
    //     });

    //     assert!(sandbox.path().join("out/file.txt").exists());
    // }

    // #[test]
    // fn unpacks_zip() {
    //     let sandbox = create_sandbox("zip");
    //     let plugin = create_extension("test", sandbox.path());

    //     plugin.execute_extension(ExecuteExtensionInput {
    //         args: vec![
    //             "--src".into(),
    //             "./archive.zip".into(),
    //             "--dest".into(),
    //             "./out".into(),
    //         ],
    //         context: plugin.create_context(sandbox.path()),
    //     });

    //     assert!(sandbox.path().join("out/file.txt").exists());
    // }

    //  #[test]
    // fn downloads_and_unpacks_tar() {
    //     let sandbox = create_empty_sandbox();
    //     let plugin = create_extension("test", sandbox.path());

    //     plugin.execute_extension(ExecuteExtensionInput {
    //         args: vec![
    //             "--src".into(),
    //             "https://github.com/moonrepo/moon/archive/refs/tags/v1.0.0.tar.gz".into(),
    //             "--dest".into(),
    //             "./out".into(),
    //         ],
    //         context: plugin.create_context(sandbox.path()),
    //     });

    //     assert!(sandbox.path().join(".moon/temp/v1.0.0.zip").exists());
    //     assert!(sandbox.path().join("out/README.md").exists());
    // }

    // #[test]
    // fn downloads_and_unpacks_zip() {
    //     let sandbox = create_empty_sandbox();
    //     let plugin = create_extension("test", sandbox.path());

    //     plugin.execute_extension(ExecuteExtensionInput {
    //         args: vec![
    //             "--src".into(),
    //             "https://github.com/moonrepo/moon/archive/refs/tags/v1.0.0.zip".into(),
    //             "--dest".into(),
    //             "./out".into(),
    //         ],
    //         context: plugin.create_context(sandbox.path()),
    //     });

    //     assert!(sandbox.path().join(".moon/temp/v1.0.0.zip").exists());
    //     assert!(sandbox.path().join("out/README.md").exists());
    // }
}
