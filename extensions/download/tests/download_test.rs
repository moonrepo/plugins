use moon_pdk_test_utils::{ExecuteExtensionInput, create_empty_moon_sandbox};
use std::fs;

mod download_extension {
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

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "A valid URL is required for downloading.")]
    async fn errors_if_not_a_url() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec!["--url".into(), "invalid".into()],
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
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                    "--dest".into(),
                    "./dest".into(),
                ],
                ..Default::default()
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_file() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                    "--dest".into(),
                    ".".into(),
                ],
                ..Default::default()
            })
            .await;

        let file = sandbox.path().join("README.md");

        assert!(file.exists());
        assert_eq!(fs::metadata(file).unwrap().len(), 4123);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_file_to_subdir() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                    "--dest".into(),
                    "./sub/dir".into(),
                ],
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join("sub/dir/README.md").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_file_with_custom_name() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

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
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join("sub/dir/moon.md").exists());
    }
}
