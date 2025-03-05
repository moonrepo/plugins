mod utils;

use moon_pdk::HashTaskContentsInput;
use moon_pdk_test_utils::{create_empty_moon_sandbox, create_moon_sandbox};
use serde_json::json;
use utils::{create_project, create_task};

mod run_task {
    use super::*;

    mod hash_task_contents {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_an_empty_array_if_no_configs() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .hash_task_contents(HashTaskContentsInput {
                    project: create_project("a"),
                    task: create_task("a:build"),
                    ..Default::default()
                })
                .await;

            assert!(output.contents.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_nothing_from_config_with_no_options() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .hash_task_contents(HashTaskContentsInput {
                    project: create_project("no-options"),
                    task: create_task("no-options:build"),
                    ..Default::default()
                })
                .await;

            // from root options
            assert_eq!(output.contents, vec![json!({ "module": "nodenext" })]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_from_extended_root_options() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .hash_task_contents(HashTaskContentsInput {
                    project: create_project("only-extend-root-options"),
                    task: create_task("only-extend-root-options:build"),
                    ..Default::default()
                })
                .await;

            assert_eq!(output.contents, vec![json!({ "module": "nodenext" })]);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_options_from_project() {
            let sandbox = create_moon_sandbox("hashing");
            let plugin = sandbox.create_toolchain("typescript").await;

            let output = plugin
                .hash_task_contents(HashTaskContentsInput {
                    project: create_project("with-options"),
                    task: create_task("with-options:build"),
                    ..Default::default()
                })
                .await;

            assert_eq!(
                output.contents,
                vec![json!({ "module": "nodenext", "target": "es2020" })]
            );
        }
    }
}
