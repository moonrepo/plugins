#[cfg(not(windows))]
mod python_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("python-test", ["python", "pip"]);
}
