use proto_pdk_test_utils::*;

mod kotlin_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("kotlin-test", ["kotlinc"]);
}
