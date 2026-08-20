use proto_pdk_test_utils::*;

mod swift_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("swift-test", ["swift", "swiftc", "sourcekit-lsp"]);
}
