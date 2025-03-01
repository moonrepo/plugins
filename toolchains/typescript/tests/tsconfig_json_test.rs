use moon_pdk::VirtualPath;
use starbase_sandbox::{create_empty_sandbox, locate_fixture};
use starbase_utils::{
    json::{self, JsonValue},
    string_vec,
};
use std::path::PathBuf;
use typescript_toolchain::tsconfig_json::TsConfigJson as TsConfigJsonContainer;
use typescript_tsconfig_json::*;

mod tsconfig_json {
    use super::*;

    #[test]
    fn preserves_when_saving() {
        let json = "{\n  \"compilerOptions\": {},\n  \"files\": [\n    \"**/*\"\n  ]\n}\n";

        let sandbox = create_empty_sandbox();
        sandbox.create_file("tsconfig.json", json);

        let config_path = sandbox.path().join("tsconfig.json");
        let mut tsc =
            TsConfigJsonContainer::load(VirtualPath::OnlyReal(config_path.clone())).unwrap();

        // Trigger dirty
        tsc.dirty.push("unknown".into());
        tsc.save().unwrap();

        assert_eq!(std::fs::read_to_string(config_path).unwrap(), json);
    }

    #[test]
    fn serializes_special_fields() {
        let actual = TsConfigJson {
            compiler_options: Some(CompilerOptions {
                module: Some(ModuleField::EsNext),
                module_resolution: Some(ModuleResolutionField::Node16),
                jsx: Some(JsxField::ReactJsxdev),
                target: Some(TargetField::Es6),
                lib: Some(string_vec![
                    "dom",
                    "es2015.generator",
                    "es2016.array.include",
                    "es2017.sharedmemory",
                    "es2018.intl",
                    "es2019.symbol",
                    "es2020.symbol.wellknown",
                    "es2021.weakref",
                ]),
                ..CompilerOptions::default()
            }),
            ..TsConfigJson::default()
        };

        let expected = serde_json::json!({
            "compilerOptions": {
                "jsx": "react-jsxdev",
                "lib": [
                    "dom",
                    "es2015.generator",
                    "es2016.array.include",
                    "es2017.sharedmemory",
                    "es2018.intl",
                    "es2019.symbol",
                    "es2020.symbol.wellknown",
                    "es2021.weakref",
                ],
                "module": "esnext",
                "moduleResolution": "node16",
                "target": "es6",
            },
        });

        assert_eq!(
            serde_json::to_string(&actual).unwrap(),
            serde_json::to_string(&expected).unwrap(),
        );
    }

    #[test]
    fn deserializes_special_fields() {
        let actual = serde_json::json!({
            "compilerOptions": {
                "jsx": "react-native",
                "lib": [
                    "dom",
                    "es2015.collection",
                    "es2016",
                    "es2017.typedarrays",
                    "es2018.promise",
                    "es2019.string",
                    "es2020",
                    "es2021.weakref",
                ],
                "module": "es2015",
                "moduleResolution": "classic",
                "target": "esnext",
            },
        });

        let expected = TsConfigJson {
            compiler_options: Some(CompilerOptions {
                jsx: Some(JsxField::ReactNative),
                lib: Some(string_vec![
                    "dom",
                    "es2015.collection",
                    "es2016",
                    "es2017.typedarrays",
                    "es2018.promise",
                    "es2019.string",
                    "es2020",
                    "es2021.weakref",
                ]),
                module: Some(ModuleField::Es2015),
                module_resolution: Some(ModuleResolutionField::Classic),
                target: Some(TargetField::EsNext),
                ..CompilerOptions::default()
            }),
            ..TsConfigJson::default()
        };

        let actual_typed: TsConfigJson = serde_json::from_value(actual).unwrap();

        assert_eq!(actual_typed, expected);
    }

    #[test]
    fn merge_two_configs() {
        let json_1 = r#"{"compilerOptions": {"jsx": "react", "noEmit": true}}"#;
        let json_2 = r#"{"compilerOptions": {"jsx": "preserve", "removeComments": true}}"#;

        let value1: JsonValue = serde_json::from_str(json_1).unwrap();
        let value2: JsonValue = serde_json::from_str(json_2).unwrap();

        let new_value = json::merge(&value1, &value2);
        let config: TsConfigJson = serde_json::from_value(new_value).unwrap();

        assert_eq!(
            config.clone().compiler_options.unwrap().jsx,
            Some(JsxField::Preserve)
        );

        assert_eq!(config.clone().compiler_options.unwrap().no_emit, Some(true));
        assert_eq!(
            config
                .compiler_options
                .unwrap()
                .other_fields
                .get("removeComments"),
            Some(&JsonValue::Bool(true))
        );
    }

    #[test]
    fn parse_basic_file() {
        let fixture = locate_fixture("configs");
        let tsc = TsConfigJsonContainer::load(VirtualPath::OnlyReal(
            fixture.join("tsconfig.default.json"),
        ))
        .unwrap();

        assert_eq!(
            tsc.data.compiler_options.clone().unwrap().target,
            Some(TargetField::Es5)
        );
        assert_eq!(
            tsc.data.compiler_options.clone().unwrap().module,
            Some(ModuleField::CommonJs)
        );
        assert_eq!(tsc.data.compiler_options.unwrap().strict, Some(true));
    }

    mod add_project_ref {
        use super::*;

        #[test]
        fn adds_if_not_set() {
            let mut tsc = TsConfigJsonContainer {
                path: VirtualPath::OnlyReal(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert_eq!(tsc.data.references, None);

            assert!(
                tsc.add_project_ref(
                    &VirtualPath::OnlyReal(PathBuf::from("/sibling")),
                    "tsconfig.json"
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.references.unwrap(),
                vec![ProjectReference {
                    path: "../sibling".into(),
                    prepend: None,
                }]
            );
        }

        #[test]
        fn doesnt_add_if_set() {
            let mut tsc = TsConfigJsonContainer {
                data: TsConfigJson {
                    references: Some(vec![ProjectReference {
                        path: "../sibling".into(),
                        prepend: None,
                    }]),
                    ..TsConfigJson::default()
                },
                path: VirtualPath::OnlyReal(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert!(
                !tsc.add_project_ref(
                    &VirtualPath::OnlyReal(PathBuf::from("/sibling")),
                    "tsconfig.json"
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.references.unwrap(),
                vec![ProjectReference {
                    path: "../sibling".into(),
                    prepend: None,
                }]
            );
        }

        #[test]
        fn includes_custom_config_name() {
            let mut tsc = TsConfigJsonContainer {
                data: TsConfigJson {
                    ..TsConfigJson::default()
                },
                path: VirtualPath::OnlyReal(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert_eq!(tsc.data.references, None);

            assert!(
                tsc.add_project_ref(
                    &VirtualPath::OnlyReal(PathBuf::from("/sibling")),
                    "tsconfig.ref.json"
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.references.unwrap(),
                vec![ProjectReference {
                    path: "../sibling/tsconfig.ref.json".into(),
                    prepend: None,
                }]
            );
        }

        #[cfg(windows)]
        #[test]
        fn forces_forward_slash() {
            let mut tsc = TsConfigJsonContainer {
                data: TsConfigJson {
                    ..TsConfigJson::default()
                },
                path: VirtualPath::OnlyReal(PathBuf::from("C:\\base\\dir\\tsconfig.json")),
                ..Default::default()
            };

            assert_eq!(tsc.data.references, None);

            assert!(
                tsc.add_project_ref(
                    &VirtualPath::OnlyReal(PathBuf::from("C:\\base\\sibling")),
                    "tsconfig.json"
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.references.unwrap(),
                vec![ProjectReference {
                    path: "../sibling".into(),
                    prepend: None,
                }]
            );
        }

        #[test]
        fn appends_and_sorts_list() {
            let mut tsc = TsConfigJsonContainer {
                data: TsConfigJson {
                    references: Some(vec![ProjectReference {
                        path: "../sister".into(),
                        prepend: None,
                    }]),
                    ..TsConfigJson::default()
                },
                path: VirtualPath::OnlyReal(PathBuf::from("/base/tsconfig.json")),
                ..Default::default()
            };

            assert!(
                tsc.add_project_ref(
                    &VirtualPath::OnlyReal(PathBuf::from("/brother")),
                    "tsconfig.json"
                )
                .unwrap()
            );

            assert_eq!(
                tsc.data.references.unwrap(),
                vec![
                    ProjectReference {
                        path: "../brother".into(),
                        prepend: None,
                    },
                    ProjectReference {
                        path: "../sister".into(),
                        prepend: None,
                    }
                ]
            );
        }
    }

    mod update_compiler_options {
        use super::*;

        #[test]
        fn creates_if_none_and_returns_true() {
            let mut tsc = TsConfigJsonContainer::default();

            let updated = tsc.update_compiler_options(|options| {
                options.out_dir = Some("./test".into());
                true
            });

            assert!(updated);
            assert_eq!(
                tsc.data
                    .compiler_options
                    .as_ref()
                    .unwrap()
                    .out_dir
                    .as_ref()
                    .unwrap(),
                &CompilerPath::from("./test")
            )
        }

        #[test]
        fn doesnt_create_if_none_and_returns_false() {
            let mut tsc = TsConfigJsonContainer::default();

            let updated = tsc.update_compiler_options(|options| {
                options.out_dir = Some("./test".into());
                false
            });

            assert!(!updated);
            assert_eq!(tsc.data.compiler_options, None)
        }

        #[test]
        fn can_update_existing() {
            let mut tsc = TsConfigJsonContainer {
                data: TsConfigJson {
                    compiler_options: Some(CompilerOptions {
                        out_dir: Some("./old".into()),
                        ..CompilerOptions::default()
                    }),
                    ..TsConfigJson::default()
                },
                ..Default::default()
            };

            let updated = tsc.update_compiler_options(|options| {
                options.out_dir = Some("./new".into());
                true
            });

            assert!(updated);
            assert_eq!(
                tsc.data
                    .compiler_options
                    .as_ref()
                    .unwrap()
                    .out_dir
                    .as_ref()
                    .unwrap(),
                &CompilerPath::from("./new")
            )
        }

        mod paths {
            use super::*;

            #[test]
            fn sets_if_none() {
                let mut tsc = TsConfigJsonContainer::default();

                let updated = tsc.update_compiler_option_paths(CompilerOptionsPathsMap::from_iter(
                    [("alias".into(), vec![CompilerPath::from("index.ts")])],
                ));

                assert!(updated);
                assert_eq!(
                    *tsc.data
                        .compiler_options
                        .as_ref()
                        .unwrap()
                        .paths
                        .as_ref()
                        .unwrap()
                        .get("alias")
                        .unwrap(),
                    vec![CompilerPath::from("index.ts")]
                );
            }

            #[test]
            fn sets_multiple() {
                let mut tsc = TsConfigJsonContainer::default();

                let updated =
                    tsc.update_compiler_option_paths(CompilerOptionsPathsMap::from_iter([
                        ("one".into(), vec![CompilerPath::from("one.ts")]),
                        ("two".into(), vec![CompilerPath::from("two.ts")]),
                        ("three".into(), vec![CompilerPath::from("three.ts")]),
                    ]));

                assert!(updated);
                assert_eq!(
                    tsc.data
                        .compiler_options
                        .as_ref()
                        .unwrap()
                        .paths
                        .as_ref()
                        .unwrap()
                        .len(),
                    3
                );
            }

            #[test]
            fn overrides_existing_value() {
                let mut tsc = TsConfigJsonContainer {
                    data: TsConfigJson {
                        compiler_options: Some(CompilerOptions {
                            paths: Some(CompilerOptionsPathsMap::from_iter([(
                                "alias".into(),
                                vec![CompilerPath::from("old.ts")],
                            )])),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                };

                let updated = tsc.update_compiler_option_paths(CompilerOptionsPathsMap::from_iter(
                    [("alias".into(), vec![CompilerPath::from("new.ts")])],
                ));

                assert!(updated);
                assert_eq!(
                    *tsc.data
                        .compiler_options
                        .as_ref()
                        .unwrap()
                        .paths
                        .as_ref()
                        .unwrap()
                        .get("alias")
                        .unwrap(),
                    vec![CompilerPath::from("new.ts")]
                );
            }

            #[test]
            fn doesnt_overrides_same_value() {
                let mut tsc = TsConfigJsonContainer {
                    data: TsConfigJson {
                        compiler_options: Some(CompilerOptions {
                            paths: Some(CompilerOptionsPathsMap::from_iter([(
                                "alias".into(),
                                vec![CompilerPath::from("./src"), CompilerPath::from("./other")],
                            )])),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                };

                let updated =
                    tsc.update_compiler_option_paths(CompilerOptionsPathsMap::from_iter([(
                        "alias".into(),
                        vec![CompilerPath::from("./src"), CompilerPath::from("./other")],
                    )]));

                assert!(!updated);

                let updated =
                    tsc.update_compiler_option_paths(CompilerOptionsPathsMap::from_iter([(
                        "alias".into(),
                        vec![CompilerPath::from("./other"), CompilerPath::from("./src")],
                    )]));

                assert!(!updated);
            }
        }
    }
}
