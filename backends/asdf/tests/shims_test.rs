#[cfg(not(windows))]
mod asdf_backend {
    use proto_pdk_test_utils::*;

    generate_shims_test!("asdf:zig", ["zig"]);
}
