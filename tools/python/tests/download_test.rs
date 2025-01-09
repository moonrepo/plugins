use proto_pdk_test_utils::*;

mod python_tool {
    use super::*;

    generate_download_install_tests!("python-test", "3.10.0");
}
