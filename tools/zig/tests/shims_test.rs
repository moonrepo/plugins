use proto_pdk_test_utils::*;

mod zig_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("zig-test", ["zig"]);
}
