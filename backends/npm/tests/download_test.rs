#[cfg(not(windows))]
mod asdf_backend {
    use proto_pdk_test_utils::*;

    generate_native_install_tests!("asdf:zig", "0.13.0");
}
