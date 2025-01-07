use moon_pdk_test_utils::{create_extension, ExecuteExtensionInput};
use starbase_sandbox::{assert_snapshot, create_sandbox};
use std::fs;

mod migrate_turborepo_extension {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn converts_basic_root_file() {
        let sandbox = create_sandbox("root-only");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert!(!sandbox.path().join("turbo.json").exists());
        assert!(sandbox.path().join(".moon/tasks/node.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/tasks/node.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn converts_project_files() {
        let sandbox = create_sandbox("monorepo");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert!(!sandbox.path().join("turbo.json").exists());
        assert!(!sandbox.path().join("client/turbo.json").exists());
        assert!(!sandbox.path().join("server/turbo.json").exists());
        assert!(sandbox.path().join(".moon/tasks/node.yml").exists());
        assert!(sandbox.path().join("client/moon.yml").exists());
        assert!(sandbox.path().join("server/moon.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/tasks/node.yml")).unwrap());
        assert_snapshot!(fs::read_to_string(sandbox.path().join("client/moon.yml")).unwrap());
        assert_snapshot!(fs::read_to_string(sandbox.path().join("server/moon.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn can_force_bun_instead_of_node() {
        let sandbox = create_sandbox("monorepo");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec!["--bun".into()],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert!(!sandbox.path().join("turbo.json").exists());
        assert!(!sandbox.path().join("client/turbo.json").exists());
        assert!(!sandbox.path().join("server/turbo.json").exists());
        assert!(!sandbox.path().join(".moon/tasks/node.yml").exists());
        assert!(sandbox.path().join(".moon/tasks/bun.yml").exists());
        assert!(sandbox.path().join("client/moon.yml").exists());
        assert!(sandbox.path().join("server/moon.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/tasks/bun.yml")).unwrap());
        assert_snapshot!(fs::read_to_string(sandbox.path().join("client/moon.yml")).unwrap());
        assert_snapshot!(fs::read_to_string(sandbox.path().join("server/moon.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn converts_to_a_root_project() {
        let sandbox = create_sandbox("root-project");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert!(!sandbox.path().join("turbo.json").exists());
        assert!(!sandbox.path().join(".moon/tasks/node.yml").exists());
        assert!(sandbox.path().join("moon.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn merges_with_existing_root_tasks() {
        let sandbox = create_sandbox("root-merge-existing");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/tasks/node.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_no_pipeline() {
        let sandbox = create_sandbox("missing-pipeline");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert!(!sandbox.path().join("turbo.json").exists());
        assert!(!sandbox.path().join(".moon/tasks/node.yml").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "Unable to migrate task as package client does not exist.")]
    async fn errors_if_a_task_points_to_an_unknown_project() {
        let sandbox = create_sandbox("error-missing-project");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "Unable to migrate task as package client does not exist.")]
    async fn errors_if_a_dependson_points_to_an_unknown_project() {
        let sandbox = create_sandbox("error-missing-project-deps");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;
    }
}
