#[cfg(not(windows))]
mod npm_backend {
    use proto_pdk_test_utils::*;

    generate_shims_test!("npm:typescript", ["tsc", "tsserver"]);
}
