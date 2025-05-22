#[cfg(unix)]
mod python_poetry_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("poetry-test", ["poetry"]);
}
