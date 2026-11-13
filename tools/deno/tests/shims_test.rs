#[cfg(not(windows))]
mod deno_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("deno-test", ["deno"]);
}
