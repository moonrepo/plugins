use cargo_backend::{CargoBackendConfig, CargoToolConfig};
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

    // https://github.com/kbknapp/cargo-outdated/blob/master/Cargo.toml

    mod features {
        use super::*;

        generate_native_install_tests!("cargo:cargo-outdated", "0.17.0", None, |cfg| {
            cfg.tool_config(CargoToolConfig {
                features: vec!["debug".into()],
                no_default_features: true,
                ..Default::default()
            });
        });
    }

    mod bin {
        use super::*;

        generate_native_install_tests!("cargo:cargo-outdated", "0.17.0", None, |cfg| {
            cfg.tool_config(CargoToolConfig {
                bin: Some("cargo-outdated".into()),
                ..Default::default()
            });
        });
    }

    mod git {
        use super::*;

        generate_native_install_tests!("cargo:cargo-outdated", "0.17.0", None, |cfg| {
            cfg.tool_config(CargoToolConfig {
                git_url: Some("https://github.com/kbknapp/cargo-outdated.git".into()),
                ..Default::default()
            });
        });
    }
}
