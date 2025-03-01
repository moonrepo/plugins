use moon_pdk_test_utils::{ExecuteExtensionInput, create_moon_sandbox};
use starbase_sandbox::assert_snapshot;
use std::fs;

mod migrate_turborepo_extension {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn converts_basic_root_file() {
        let sandbox = create_moon_sandbox("root-only");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;

        assert!(!sandbox.path().join("turbo.json").exists());
        assert!(sandbox.path().join(".moon/tasks/node.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/tasks/node.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn converts_project_files() {
        let sandbox = create_moon_sandbox("monorepo");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
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
        let sandbox = create_moon_sandbox("monorepo");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec!["--bun".into()],
                ..Default::default()
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
        let sandbox = create_moon_sandbox("root-project");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;

        assert!(!sandbox.path().join("turbo.json").exists());
        assert!(!sandbox.path().join(".moon/tasks/node.yml").exists());
        assert!(sandbox.path().join("moon.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn merges_with_existing_root_tasks() {
        let sandbox = create_moon_sandbox("root-merge-existing");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/tasks/node.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn supports_no_pipeline() {
        let sandbox = create_moon_sandbox("missing-pipeline");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;

        assert!(!sandbox.path().join("turbo.json").exists());
        assert!(!sandbox.path().join(".moon/tasks/node.yml").exists());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "Unable to migrate task as package client does not exist.")]
    async fn errors_if_a_task_points_to_an_unknown_project() {
        let sandbox = create_moon_sandbox("error-missing-project");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "Unable to migrate task as package client does not exist.")]
    async fn errors_if_a_dependson_points_to_an_unknown_project() {
        let sandbox = create_moon_sandbox("error-missing-project-deps");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;
    }
}
