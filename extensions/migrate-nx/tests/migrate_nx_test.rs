use moon_pdk_test_utils::{create_extension, ExecuteExtensionInput};
use starbase_sandbox::{assert_snapshot, create_empty_sandbox, create_sandbox};
use std::fs;

mod migrate_nx_extension {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn converts_root_files() {
        let sandbox = create_sandbox("root");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
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
        let sandbox = create_sandbox("nx-executors");
        let plugin = create_extension("test", sandbox.path());

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec![],
                context: plugin.create_context(sandbox.path()),
            })
            .await;

        assert!(!sandbox.path().join("project.json").exists());
        assert!(sandbox.path().join("moon.yml").exists());

        assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
    }

    mod nx_json {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_named_inputs() {
            let sandbox = create_sandbox("root-inputs");
            let plugin = create_extension("test", sandbox.path());

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: vec![],
                    context: plugin.create_context(sandbox.path()),
                })
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
            let sandbox = create_empty_sandbox();
            let plugin = create_extension("test", sandbox.path());

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: vec![],
                    context: plugin.create_context(sandbox.path()),
                })
                .await;

            assert_snapshot!(
                fs::read_to_string(sandbox.path().join(".moon/workspace.yml")).unwrap()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn inherits_layout() {
            let sandbox = create_empty_sandbox();
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

            let plugin = create_extension("test", sandbox.path());

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: vec![],
                    context: plugin.create_context(sandbox.path()),
                })
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
            let sandbox = create_sandbox("projects");
            let plugin = create_extension("test", sandbox.path());

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: vec![],
                    context: plugin.create_context(sandbox.path()),
                })
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
            let sandbox = create_sandbox("project-name-deps");
            let plugin = create_extension("test", sandbox.path());

            dbg!(sandbox.path());

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: vec![],
                    context: plugin.create_context(sandbox.path()),
                })
                .await;

            assert!(!sandbox.path().join("project.json").exists());
            assert!(sandbox.path().join("moon.yml").exists());

            assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_type_and_tags() {
            let sandbox = create_sandbox("project-type-tags");
            let plugin = create_extension("test", sandbox.path());

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: vec![],
                    context: plugin.create_context(sandbox.path()),
                })
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
            let sandbox = create_sandbox("project-inputs");
            let plugin = create_extension("test", sandbox.path());

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: vec![],
                    context: plugin.create_context(sandbox.path()),
                })
                .await;

            assert!(!sandbox.path().join("project.json").exists());
            assert!(sandbox.path().join("moon.yml").exists());

            assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn converts_targets() {
            let sandbox = create_sandbox("project-targets");
            let plugin = create_extension("test", sandbox.path());

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: vec![],
                    context: plugin.create_context(sandbox.path()),
                })
                .await;

            assert!(!sandbox.path().join("project.json").exists());
            assert!(sandbox.path().join("moon.yml").exists());

            assert_snapshot!(fs::read_to_string(sandbox.path().join("moon.yml")).unwrap());
        }
    }
}
