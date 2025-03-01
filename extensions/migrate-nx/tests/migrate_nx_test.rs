use moon_pdk_test_utils::{ExecuteExtensionInput, create_empty_moon_sandbox, create_moon_sandbox};
use starbase_sandbox::assert_snapshot;
use std::fs;

mod migrate_nx_extension {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn converts_root_files() {
        let sandbox = create_moon_sandbox("root");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;

        assert!(!sandbox.path().join("nx.json").exists());
        assert!(!sandbox.path().join("workspace.json").exists());
        assert!(sandbox.path().join(".moon/tasks/node.yml").exists());
        assert!(sandbox.path().join(".moon/workspace.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/tasks/node.yml")).unwrap());
        assert_snapshot!(fs::read_to_string(sandbox.path().join(".moon/workspace.yml")).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn converts_nx_builtin_executors() {
        let sandbox = create_moon_sandbox("nx-executors");
        let plugin = sandbox.create_extension("test").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;

        assert!(!sandbox.path().join("project.json").exists());
        assert!(sandbox.path().join("moon.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
    }

    mod nx_json {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_named_inputs() {
            let sandbox = create_moon_sandbox("root-inputs");
            let plugin = sandbox.create_extension("test").await;

            plugin
                .execute_extension(ExecuteExtensionInput::default())
                .await;

            assert!(!sandbox.path().join("nx.json").exists());
            assert!(sandbox.path().join(".moon/tasks/node.yml").exists());

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".moon/tasks/node.yml")).unwrap()
            );
        }
    }

    mod workspace_projects {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn uses_defaults() {
            let sandbox = create_empty_moon_sandbox();
            let plugin = sandbox.create_extension("test").await;

            plugin
                .execute_extension(ExecuteExtensionInput::default())
                .await;

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".moon/workspace.yml")).unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_layout() {
            let sandbox = create_empty_moon_sandbox();
            sandbox.create_file(
                "nx.json",
                r#"
{
  "workspaceLayout": {
    "appsDir": "applications",
    "libsDir": "libraries"
  }
}"#,
            );

            let plugin = sandbox.create_extension("test").await;

            plugin
                .execute_extension(ExecuteExtensionInput::default())
                .await;

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".moon/workspace.yml")).unwrap()
            );
        }
    }

    mod projects {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_project_json() {
            let sandbox = create_moon_sandbox("projects");
            let plugin = sandbox.create_extension("test").await;

            plugin
                .execute_extension(ExecuteExtensionInput::default())
                .await;

            assert!(!sandbox.path().join("nx.json").exists());
            assert!(!sandbox.path().join("bar/project.json").exists());
            assert!(!sandbox.path().join("baz/project.json").exists());
            assert!(!sandbox.path().join("foo/project.json").exists());
            assert!(sandbox.path().join("bar/moon.yml").exists());
            assert!(sandbox.path().join("baz/moon.yml").exists());
            assert!(sandbox.path().join("foo/moon.yml").exists());

            assert_snapshot!(fs::read_to_string(sandbox.path().join("bar/moon.yml")).unwrap());
            assert_snapshot!(fs::read_to_string(sandbox.path().join("baz/moon.yml")).unwrap());
            assert_snapshot!(fs::read_to_string(sandbox.path().join("foo/moon.yml")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_name_and_implicit_deps() {
            let sandbox = create_moon_sandbox("project-name-deps");
            let plugin = sandbox.create_extension("test").await;

            plugin
                .execute_extension(ExecuteExtensionInput::default())
                .await;

            assert!(!sandbox.path().join("project.json").exists());
            assert!(sandbox.path().join("moon.yml").exists());

            assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_type_and_tags() {
            let sandbox = create_moon_sandbox("project-type-tags");
            let plugin = sandbox.create_extension("test").await;

            plugin
                .execute_extension(ExecuteExtensionInput::default())
                .await;

            assert!(!sandbox.path().join("app/project.json").exists());
            assert!(!sandbox.path().join("lib/project.json").exists());
            assert!(sandbox.path().join("app/moon.yml").exists());
            assert!(sandbox.path().join("lib/moon.yml").exists());

            assert_snapshot!(fs::read_to_string(sandbox.path().join("app/moon.yml")).unwrap());
            assert_snapshot!(fs::read_to_string(sandbox.path().join("lib/moon.yml")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_named_inputs() {
            let sandbox = create_moon_sandbox("project-inputs");
            let plugin = sandbox.create_extension("test").await;

            plugin
                .execute_extension(ExecuteExtensionInput::default())
                .await;

            assert!(!sandbox.path().join("project.json").exists());
            assert!(sandbox.path().join("moon.yml").exists());

            assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_targets() {
            let sandbox = create_moon_sandbox("project-targets");
            let plugin = sandbox.create_extension("test").await;

            plugin
                .execute_extension(ExecuteExtensionInput::default())
                .await;

            assert!(!sandbox.path().join("project.json").exists());
            assert!(sandbox.path().join("moon.yml").exists());

            assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
        }
    }
}
