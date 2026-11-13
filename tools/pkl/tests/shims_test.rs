use proto_pdk_test_utils::*;

mod pkl_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("pkl-test", ["pkl"]);
}
