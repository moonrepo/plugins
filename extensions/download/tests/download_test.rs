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
    #[should_panic(expected = "A valid URL is required for downloading.")]
    async fn errors_if_missing_url_scheme() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec!["--url".into(), "httpfoo".into()],
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
        assert!(fs::metadata(file).unwrap().len() > 0);
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

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_file_without_query_in_name() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md?raw=true#top"
                        .into(),
                ],
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join("README.md").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_file_to_absolute_dest() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![
                    "--url".into(),
                    "https://raw.githubusercontent.com/moonrepo/moon/master/README.md".into(),
                    "--dest".into(),
                    sandbox.path().join("sub").to_string_lossy().to_string(),
                ],
                ..Default::default()
            })
            .await;

        assert!(sandbox.path().join("sub/README.md").exists());
    }
}
