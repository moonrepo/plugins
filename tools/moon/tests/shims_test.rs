use proto_pdk_test_utils::*;

mod moon_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("moon-test", ["moon"]);
}
