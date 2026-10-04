use proto_pdk_test_utils::*;

#[tokio::test(flavor = "multi_thread")]
async fn registers_portable_language_with_java_dependency() {
    let sandbox = create_empty_proto_sandbox();
    let plugin = sandbox.create_plugin("kotlin-test").await;
    let output = plugin
        .register_tool(RegisterToolInput {
            id: Id::raw("kotlin"),
        })
        .await;

    assert_eq!(output.name, "Kotlin");
    assert!(matches!(output.type_of, PluginType::Language));
    assert_eq!(output.requires, ["java"]);
    assert!(output.lock_options.ignore_os_arch);
    assert_eq!(output.minimum_proto_version, Some(Version::new(0, 60, 0)));
    assert_eq!(
        output.plugin_version.unwrap().to_string(),
        env!("CARGO_PKG_VERSION")
    );

    let schema: DefineToolConfigOutput = plugin
        .tool
        .plugin
        .call_func_with(PluginFunction::DefineToolConfig, ())
        .await
        .unwrap();
    assert!(format!("{schema:?}").contains("dist-url"));
}
