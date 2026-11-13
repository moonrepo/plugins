#[cfg(not(windows))]
mod python_uv_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("uv-test", ["uv", "uvx"]);
}
