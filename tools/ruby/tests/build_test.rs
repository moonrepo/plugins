#[cfg(unix)]
mod ruby_tool {
    use proto_pdk_test_utils::*;

    generate_build_install_tests!("ruby-test", "3.4.0");
}
