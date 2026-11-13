#[cfg(not(windows))]
mod zig_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("zig-test", ["zig"]);
}
