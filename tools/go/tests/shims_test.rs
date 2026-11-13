#[cfg(not(windows))]
mod go_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("go-test", ["go"]);
}
