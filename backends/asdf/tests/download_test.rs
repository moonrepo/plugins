#[cfg(not(windows))]
mod asdf_backend {
    use proto_pdk_test_utils::*;

    generate_native_install_tests!("asdf:act", "asdf:0.2.70");
}
