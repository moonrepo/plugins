#[cfg(not(windows))]
mod schema_tool {
    use proto_pdk_test_utils::*;
    use starbase_sandbox::locate_fixture;

    generate_shims_test!(
        "schema-test",
        ["schema-test"],
        Some(locate_fixture("schemas").join("base.toml"))
    );
}
