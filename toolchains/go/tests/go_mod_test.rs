use go_toolchain::go_mod::*;
use starbase_sandbox::create_sandbox;
use std::fs;

mod go_mod {
    use super::*;

    #[test]
    fn parses_basic() {
        let sandbox = create_sandbox("mod-files");
        let go_mod =
            parse_go_mod(fs::read_to_string(sandbox.path().join("basic.mod")).unwrap()).unwrap();

        assert_eq!(go_mod.module, "github.com/org/repo");
        assert_eq!(go_mod.go.unwrap(), "1.24");
        assert_eq!(go_mod.require.len(), 22);
    }

    #[test]
    fn parses_advanced() {
        let sandbox = create_sandbox("mod-files");
        let go_mod =
            parse_go_mod(fs::read_to_string(sandbox.path().join("advanced.mod")).unwrap()).unwrap();

        assert_eq!(go_mod.module, "github.com/org/repo");
        assert_eq!(
            go_mod.comment,
            vec!["Deprecated: use v2 instead", "Other comment"]
        );
        assert_eq!(go_mod.go.unwrap(), "1.24.0");
        assert_eq!(go_mod.toolchain.unwrap(), "go1.21.0");
        assert_eq!(
            go_mod.require,
            vec![
                ModuleDependency {
                    module: Module {
                        module_path: "example.com/new/thing/v2".into(),
                        version: "v2.3.4".into(),
                    },
                    indirect: false
                },
                ModuleDependency {
                    module: Module {
                        module_path: "example.com/old/thing".into(),
                        version: "v1.2.3".into(),
                    },
                    indirect: false
                }
            ]
        );
        assert_eq!(
            go_mod.exclude,
            vec![ModuleDependency {
                module: Module {
                    module_path: "example.com/old/thing".into(),
                    version: "v1.2.3".into(),
                },
                indirect: false
            }]
        );
        assert_eq!(
            go_mod.replace,
            vec![ModuleReplacement {
                module_path: "example.com/bad/thing".into(),
                version: Some("v1.4.5".into()),
                replacement: Replacement::Module(Module {
                    module_path: "example.com/good/thing".into(),
                    version: "v1.4.5".into(),
                })
            }]
        );
        assert_eq!(
            go_mod.retract,
            vec![ModuleRetract::Range("v1.9.0".into(), "v1.9.5".into())]
        );
    }
}
