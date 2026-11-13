#[cfg(not(windows))]
mod zig_ls_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("zls-test", ["zls"]);
}
