use proto_pdk_test_utils::*;

fn create_metadata(id: &str) -> ToolMetadataInput {
    ToolMetadataInput { id: id.into() }
}

mod npm {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("npm-test").await;

        let metadata = plugin.register_tool(create_metadata("npm-test")).await;

        assert_eq!(metadata.name, "npm");
        assert_eq!(metadata.type_of, PluginType::DependencyManager);
        assert_eq!(metadata.plugin_version.unwrap(), env!("CARGO_PKG_VERSION"));
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
        assert_eq!(metadata.type_of, PluginType::DependencyManager);
        assert_eq!(metadata.plugin_version.unwrap(), env!("CARGO_PKG_VERSION"));
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
        assert_eq!(metadata.type_of, PluginType::DependencyManager);
        assert_eq!(metadata.plugin_version.unwrap(), env!("CARGO_PKG_VERSION"));
    }
}
