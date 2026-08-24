use proto_pdk_test_utils::*;

mod zig_ls_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("zls-test", ["zls"]);
}
