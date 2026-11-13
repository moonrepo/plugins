#[cfg(not(windows))]
mod bun_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("bun-test", ["bun", "bunx"]);
}
