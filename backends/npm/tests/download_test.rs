use proto_pdk_test_utils::*;

mod npm_backend {
    use super::*;

    generate_native_install_tests!("npm:typescript", "5.9.2");
}
