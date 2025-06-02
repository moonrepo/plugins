use moon_pdk_api::VirtualPath;
use rust_toolchain::cargo_toml::*;
use std::path::PathBuf;

mod cargo_toml {
    use super::*;

    mod set_package_msrv {
        use super::*;

        #[test]
        fn adds_if_not_set() {
            let mut manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: CargoTomlInner::new_package(),
                ..Default::default()
            };

            assert_eq!(manifest.data.package.as_ref().unwrap().rust_version, None);

            assert!(manifest.set_msrv("1.69.0").unwrap());

            assert_eq!(
                manifest.data.package.as_ref().unwrap().rust_version,
                Some(Inheritable::Set("1.69.0".into()))
            );
            assert_eq!(manifest.dirty, ["package.rust-version"]);
        }

        #[test]
        fn doesnt_add_if_empty() {
            let mut manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: CargoTomlInner::new_package(),
                ..Default::default()
            };

            assert_eq!(manifest.data.package.as_ref().unwrap().rust_version, None);

            assert!(!manifest.set_msrv("").unwrap());

            assert_eq!(manifest.data.package.as_ref().unwrap().rust_version, None);
            assert!(manifest.dirty.is_empty());
        }

        #[test]
        fn doesnt_add_if_same_value() {
            let mut manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: {
                    let mut pkg = CargoTomlInner::new_package();
                    pkg.package
                        .as_mut()
                        .unwrap()
                        .set_rust_version(Some("1.69.0".into()));
                    pkg
                },
                ..Default::default()
            };

            assert_eq!(
                manifest.data.package.as_ref().unwrap().rust_version,
                Some(Inheritable::Set("1.69.0".into()))
            );

            assert!(!manifest.set_msrv("1.69.0").unwrap());

            assert_eq!(
                manifest.data.package.as_ref().unwrap().rust_version,
                Some(Inheritable::Set("1.69.0".into()))
            );
            assert!(manifest.dirty.is_empty());
        }

        #[test]
        fn saves_field_if_set() {
            let mut manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: CargoTomlInner::new_package(),
                ..Default::default()
            };

            manifest.set_msrv("1.69.0").unwrap();

            let mut root = TomlValue::Table(Default::default());

            manifest
                .save_field("package.rust-version", &mut root)
                .unwrap();

            assert_eq!(
                root["package"]["rust-version"],
                TomlValue::String("1.69.0".into())
            );
        }

        #[test]
        fn doesnt_save_field_if_not_set() {
            let manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: CargoTomlInner::new_package(),
                ..Default::default()
            };

            let mut root = TomlValue::Table(Default::default());

            manifest
                .save_field("package.rust-version", &mut root)
                .unwrap();

            assert!(root.as_table().unwrap().is_empty());
        }
    }

    mod set_workspace_msrv {
        use super::*;

        fn get_msrv(manifest: &CargoToml) -> Option<&str> {
            manifest
                .data
                .workspace
                .as_ref()?
                .package
                .as_ref()?
                .rust_version
                .as_deref()
        }

        #[test]
        fn adds_if_not_set() {
            let mut manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: CargoTomlInner::new_workspace(),
                ..Default::default()
            };

            assert_eq!(get_msrv(&manifest), None);

            assert!(manifest.set_msrv("1.69.0").unwrap());

            assert_eq!(get_msrv(&manifest), Some("1.69.0"));
            assert_eq!(manifest.dirty, ["workspace.package.rust-version"]);
        }

        #[test]
        fn doesnt_add_if_empty() {
            let mut manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: CargoTomlInner::new_workspace(),
                ..Default::default()
            };

            assert_eq!(get_msrv(&manifest), None);

            assert!(!manifest.set_msrv("").unwrap());

            assert_eq!(get_msrv(&manifest), None);
            assert!(manifest.dirty.is_empty());
        }

        #[test]
        fn doesnt_add_if_same_value() {
            let mut manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: {
                    let mut ws = CargoTomlInner::new_workspace();
                    let pkg = ws
                        .workspace
                        .as_mut()
                        .unwrap()
                        .package
                        .get_or_insert_default();

                    pkg.rust_version = Some("1.69.0".into());
                    ws
                },
                ..Default::default()
            };

            assert_eq!(get_msrv(&manifest), Some("1.69.0"));

            assert!(!manifest.set_msrv("1.69.0").unwrap());

            assert_eq!(get_msrv(&manifest), Some("1.69.0"));
            assert!(manifest.dirty.is_empty());
        }

        #[test]
        fn saves_field_if_set() {
            let mut manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: CargoTomlInner::new_workspace(),
                ..Default::default()
            };

            manifest.set_msrv("1.69.0").unwrap();

            let mut root = TomlValue::Table(Default::default());

            manifest
                .save_field("workspace.package.rust-version", &mut root)
                .unwrap();

            assert_eq!(
                root["workspace"]["package"]["rust-version"],
                TomlValue::String("1.69.0".into())
            );
        }

        #[test]
        fn doesnt_save_field_if_not_set() {
            let manifest = CargoToml {
                path: VirtualPath::Real(PathBuf::from("/base/Cargo.toml")),
                data: CargoTomlInner::new_workspace(),
                ..Default::default()
            };

            let mut root = TomlValue::Table(Default::default());

            manifest
                .save_field("workspace.package.rust-version", &mut root)
                .unwrap();

            assert!(root.as_table().unwrap().is_empty());
        }
    }
}
