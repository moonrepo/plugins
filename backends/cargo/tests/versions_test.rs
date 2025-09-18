use proto_pdk_test_utils::*;

mod cargo_backend_versions {
    use super::*;

    generate_resolve_versions_tests!("cargo:cargo-nextest", {
        "0.9.100" => "0.9.100",
    });
}
