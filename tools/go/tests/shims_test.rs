use proto_pdk_test_utils::*;

mod go_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("go-test", ["go"]);
}
