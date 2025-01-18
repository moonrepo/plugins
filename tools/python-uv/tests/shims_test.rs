use proto_pdk_test_utils::*;

mod python_uv_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("uv-test", ["uv", "uvx"]);
}
