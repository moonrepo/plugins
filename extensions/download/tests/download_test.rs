use moon_pdk_test_utils::{create_extension, ExecuteExtensionInput};
use starbase_sandbox::create_empty_sandbox;
use std::fs;

mod download_extension {
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
    #[should_panic(expected = "A valid URL is required for downloading.")]
    async fn errors_if_not_a_url() {
        let sandbox = create_empty_sandbox();
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec!["--url".into(), "invalid".into()],
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
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                    "--dest".into(),
                    "./dest".into(),
                ],
                context: plugin.create_context(sandbox.path()),
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_file() {
        let sandbox = create_empty_sandbox();
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                    "--dest".into(),
                    ".".into(),
                ],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        let file = sandbox.path().join("README.md");

        assert!(file.exists());
        assert_eq!(fs::metadata(file).unwrap().len(), 4123);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_file_to_subdir() {
        let sandbox = create_empty_sandbox();
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                    "--dest".into(),
                    "./sub/dir".into(),
                ],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert!(sandbox.path().join("sub/dir/README.md").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_file_with_custom_name() {
        let sandbox = create_empty_sandbox();
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                    "--dest".into(),
                    "./sub/dir".into(),
                    "--name".into(),
                    "moon.md".into(),
                ],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert!(sandbox.path().join("sub/dir/moon.md").exists());
    }
}
