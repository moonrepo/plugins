#[cfg(not(windows))]
mod swift_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("swift-test", ["swift", "swiftc", "sourcekit-lsp"]);
}
