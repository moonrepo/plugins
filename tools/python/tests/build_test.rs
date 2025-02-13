#[cfg(unix)]
mod python_tool {
    use proto_pdk_test_utils::*;

    generate_build_install_tests!("python-test", "3.12.0");
}
