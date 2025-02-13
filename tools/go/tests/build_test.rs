use proto_pdk_test_utils::*;

mod go_tool {
    use super::*;

    generate_build_install_tests!("go-test", "1.21.0");
}
