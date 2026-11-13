#[cfg(not(windows))]
mod moon_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("moon-test", ["moon"]);
}
