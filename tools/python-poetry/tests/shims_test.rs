use proto_pdk_test_utils::*;

mod python_poetry_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("poetry-test", ["poetry"]);
}
