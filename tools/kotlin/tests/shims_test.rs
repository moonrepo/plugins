#[cfg(not(windows))]
mod kotlin_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("kotlin-test", ["kotlinc"]);
}
