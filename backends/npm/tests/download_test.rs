use npm_backend::NpmBackendConfig;
use proto_pdk_test_utils::*;

mod npm_backend_download {
    use super::*;

    mod with_node {
        use super::*;

        generate_native_install_tests!("npm:typescript", "5.9.2");
    }

    mod with_bun {
        use super::*;

        generate_native_install_tests!("npm:typescript", "5.9.2", None, |cfg| {
            cfg.backend_config(NpmBackendConfig { bun: true });
        });
    }
}
