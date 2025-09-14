use cargo_backend::CargoBackendConfig;
use proto_pdk_test_utils::*;

mod cargo_backend_download {
    use super::*;

    mod with_binstall {
        use super::*;

        generate_native_install_tests!("cargo:eza", "0.23.1");
    }

    mod without_binstall {
        use super::*;

        generate_native_install_tests!("cargo:eza", "0.23.1", None, |cfg| {
            cfg.backend_config(CargoBackendConfig {
                no_binstall: true,
                ..Default::default()
            });
        });
    }
}
