use proto_pdk_test_utils::*;

mod deno_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("deno-test", ["deno"]);
}
