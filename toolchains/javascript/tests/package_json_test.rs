use javascript_toolchain::package_json::PackageJson;
use moon_pdk_api::VirtualPath;
use nodejs_package_json::{PackageJson as PackageJsonInner, VersionProtocol};
use starbase_sandbox::create_empty_sandbox;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

mod package_json {
    use super::*;

    #[test]
    fn preserves_when_saving() {
        let json = "{\n  \"name\": \"test\",\n  \"version\": \"1.0.0\"\n}\n";

        let sandbox = create_empty_sandbox();
        sandbox.create_file("package.json", json);

        let config_path = sandbox.path().join("package.json");
        let mut file = PackageJson::load(VirtualPath::Real(config_path.clone())).unwrap();

        // Trigger dirty
        file.dirty.push("unknown".into());
        file.save().unwrap();

        assert_eq!(std::fs::read_to_string(config_path).unwrap(), json);
    }

    mod add_dependency {
        use super::*;

        #[test]
        fn adds() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert_eq!(tsc.data.dependencies, None);

            assert!(
                tsc.add_dependency("example", VersionProtocol::from_str("*").unwrap(), false)
                    .unwrap()
            );

            assert_eq!(
                tsc.data.dependencies.unwrap(),
                BTreeMap::from_iter([("example".into(), VersionProtocol::from_str("*").unwrap())])
            );
        }

        #[test]
        fn overwrites() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                data: PackageJsonInner {
                    dependencies: Some(BTreeMap::from_iter([
                        ("example".into(), VersionProtocol::from_str("*").unwrap()),
                        ("other".into(), VersionProtocol::from_str("*").unwrap()),
                    ])),
                    ..Default::default()
                },
                ..Default::default()
            };

            assert!(tsc.data.dependencies.is_some());

            assert!(
                tsc.add_dependency(
                    "example",
                    VersionProtocol::from_str("1.2.3").unwrap(),
                    false
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.dependencies.unwrap(),
                BTreeMap::from_iter([
                    (
                        "example".into(),
                        VersionProtocol::from_str("1.2.3").unwrap()
                    ),
                    ("other".into(), VersionProtocol::from_str("*").unwrap()),
                ])
            );
        }

        #[test]
        fn doesnt_overwrite() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                data: PackageJsonInner {
                    dependencies: Some(BTreeMap::from_iter([(
                        "example".into(),
                        VersionProtocol::from_str("*").unwrap(),
                    )])),
                    ..Default::default()
                },
                ..Default::default()
            };

            assert!(tsc.data.dependencies.is_some());

            assert!(
                !tsc.add_dependency(
                    "example",
                    VersionProtocol::from_str("1.2.3").unwrap(),
                    // Is missing check!
                    true
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.dependencies.unwrap(),
                BTreeMap::from_iter([("example".into(), VersionProtocol::from_str("*").unwrap()),])
            );
        }
    }

    mod add_dev_dependency {
        use super::*;

        #[test]
        fn adds() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert_eq!(tsc.data.dev_dependencies, None);

            assert!(
                tsc.add_dev_dependency("example", VersionProtocol::from_str("*").unwrap(), false)
                    .unwrap()
            );

            assert_eq!(
                tsc.data.dev_dependencies.unwrap(),
                BTreeMap::from_iter([("example".into(), VersionProtocol::from_str("*").unwrap())])
            );
        }

        #[test]
        fn overwrites() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                data: PackageJsonInner {
                    dev_dependencies: Some(BTreeMap::from_iter([
                        ("example".into(), VersionProtocol::from_str("*").unwrap()),
                        ("other".into(), VersionProtocol::from_str("*").unwrap()),
                    ])),
                    ..Default::default()
                },
                ..Default::default()
            };

            assert!(tsc.data.dev_dependencies.is_some());

            assert!(
                tsc.add_dev_dependency(
                    "example",
                    VersionProtocol::from_str("1.2.3").unwrap(),
                    false
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.dev_dependencies.unwrap(),
                BTreeMap::from_iter([
                    (
                        "example".into(),
                        VersionProtocol::from_str("1.2.3").unwrap()
                    ),
                    ("other".into(), VersionProtocol::from_str("*").unwrap()),
                ])
            );
        }

        #[test]
        fn doesnt_overwrite() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                data: PackageJsonInner {
                    dev_dependencies: Some(BTreeMap::from_iter([(
                        "example".into(),
                        VersionProtocol::from_str("*").unwrap(),
                    )])),
                    ..Default::default()
                },
                ..Default::default()
            };

            assert!(tsc.data.dev_dependencies.is_some());

            assert!(
                !tsc.add_dev_dependency(
                    "example",
                    VersionProtocol::from_str("1.2.3").unwrap(),
                    // Is missing check!
                    true
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.dev_dependencies.unwrap(),
                BTreeMap::from_iter([("example".into(), VersionProtocol::from_str("*").unwrap()),])
            );
        }
    }

    mod add_peer_dependency {
        use super::*;

        #[test]
        fn adds() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert_eq!(tsc.data.peer_dependencies, None);

            assert!(
                tsc.add_peer_dependency("example", VersionProtocol::from_str("*").unwrap(), false)
                    .unwrap()
            );

            assert_eq!(
                tsc.data.peer_dependencies.unwrap(),
                BTreeMap::from_iter([("example".into(), VersionProtocol::from_str("*").unwrap())])
            );
        }

        #[test]
        fn overwrites() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                data: PackageJsonInner {
                    peer_dependencies: Some(BTreeMap::from_iter([
                        ("example".into(), VersionProtocol::from_str("*").unwrap()),
                        ("other".into(), VersionProtocol::from_str("*").unwrap()),
                    ])),
                    ..Default::default()
                },
                ..Default::default()
            };

            assert!(tsc.data.peer_dependencies.is_some());

            assert!(
                tsc.add_peer_dependency(
                    "example",
                    VersionProtocol::from_str("1.2.3").unwrap(),
                    false
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.peer_dependencies.unwrap(),
                BTreeMap::from_iter([
                    (
                        "example".into(),
                        VersionProtocol::from_str("1.2.3").unwrap()
                    ),
                    ("other".into(), VersionProtocol::from_str("*").unwrap()),
                ])
            );
        }

        #[test]
        fn doesnt_overwrite() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                data: PackageJsonInner {
                    peer_dependencies: Some(BTreeMap::from_iter([(
                        "example".into(),
                        VersionProtocol::from_str("*").unwrap(),
                    )])),
                    ..Default::default()
                },
                ..Default::default()
            };

            assert!(tsc.data.peer_dependencies.is_some());

            assert!(
                !tsc.add_peer_dependency(
                    "example",
                    VersionProtocol::from_str("1.2.3").unwrap(),
                    // Is missing check!
                    true
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.peer_dependencies.unwrap(),
                BTreeMap::from_iter([("example".into(), VersionProtocol::from_str("*").unwrap()),])
            );
        }
    }

    mod set_package_manager {
        use super::*;

        #[test]
        fn sets() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert_eq!(tsc.data.package_manager, None);

            assert!(tsc.set_package_manager("value").unwrap());

            assert_eq!(tsc.data.package_manager.unwrap(), "value");
        }

        #[test]
        fn doesnt_set_if_empty() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert_eq!(tsc.data.package_manager, None);

            assert!(tsc.set_package_manager("").unwrap());

            assert_eq!(tsc.data.package_manager, None);
        }

        #[test]
        fn unsets_if_empty() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            tsc.data.package_manager = Some("value".into());

            assert!(tsc.set_package_manager("").unwrap());

            assert_eq!(tsc.data.package_manager, None);
        }

        #[test]
        fn doesnt_set_if_same_value() {
            let mut tsc = PackageJson {
                path: VirtualPath::Real(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            tsc.data.package_manager = Some("value".into());

            assert!(!tsc.set_package_manager("value").unwrap());

            assert_eq!(tsc.data.package_manager.unwrap(), "value");
        }
    }
}
