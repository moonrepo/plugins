use proto_pdk_test_utils::*;

mod bun_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("bun-test", ["bun", "bunx"]);
}
