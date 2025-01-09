use proto_pdk_test_utils::*;

mod python_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("python-test", ["python", "pip"]);
}
