use proto_pdk_test_utils::*;

fn create_metadata(id: &str) -> RegisterToolInput {
    RegisterToolInput { id: Id::raw(id) }
}

mod node_depman_tool {
    use super::*;

    mod npm {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn registers_metadata() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("npm-test").await;

            let metadata = plugin.register_tool(create_metadata("npm-test")).await;

            assert_eq!(metadata.name, "npm");
            assert!(metadata.lock_options.ignore_os_arch);
            assert_eq!(metadata.type_of, PluginType::DependencyManager);
        }
    }

    mod nub {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn registers_metadata() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("nub-test").await;

            let metadata = plugin.register_tool(create_metadata("nub-test")).await;

            assert_eq!(metadata.name, "nub");

            // Binaries are os/arch specific, so records must be scoped
            assert!(!metadata.lock_options.ignore_os_arch);
            assert_eq!(metadata.type_of, PluginType::DependencyManager);

            // Unlike the other package managers, nub does not require Node.js
            assert!(metadata.requires.is_empty());
        }
    }

    mod pnpm {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn registers_metadata() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("pnpm-test").await;

            let metadata = plugin.register_tool(create_metadata("pnpm-test")).await;

            assert_eq!(metadata.name, "pnpm");

            // v12+ binaries are os/arch specific, so records must be scoped
            assert!(!metadata.lock_options.ignore_os_arch);
            assert_eq!(metadata.type_of, PluginType::DependencyManager);
        }
    }

    mod yarn {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn registers_metadata() {
            let sandbox = create_empty_proto_sandbox();
            let plugin = sandbox.create_plugin("yarn-test").await;

            let metadata = plugin.register_tool(create_metadata("yarn-test")).await;

            assert_eq!(metadata.name, "yarn");

            // v6+ binaries are os/arch specific, so records must be scoped
            assert!(!metadata.lock_options.ignore_os_arch);
            assert_eq!(metadata.type_of, PluginType::DependencyManager);
        }
    }
}
