use go_toolchain::go_work::*;
use starbase_sandbox::create_sandbox;
use std::fs;

mod go_work {
    use super::*;

    #[test]
    fn parses_basic() {
        let sandbox = create_sandbox("work-files");
        let go_work =
            GoWork::parse(fs::read_to_string(sandbox.path().join("basic.work")).unwrap()).unwrap();

        assert_eq!(go_work.version.unwrap(), "1.23.0");
        assert_eq!(go_work.modules, vec!["a", "b"]);
    }

    #[test]
    fn parses_basic_without_trailing() {
        let sandbox = create_sandbox("work-files");

        GoWork::parse(
            fs::read_to_string(sandbox.path().join("basic.work"))
                .unwrap()
                .trim(),
        )
        .unwrap();
    }

    #[test]
    fn parses_advanced() {
        let sandbox = create_sandbox("work-files");
        let go_work =
            GoWork::parse(fs::read_to_string(sandbox.path().join("advanced.work")).unwrap())
                .unwrap();

        assert_eq!(go_work.version.unwrap(), "1.23.0");
        assert_eq!(go_work.modules, vec!["a", "b", "d", ".", "e", "f"]);
    }

    #[test]
    fn parses_advanced_without_trailing() {
        let sandbox = create_sandbox("work-files");

        GoWork::parse(
            fs::read_to_string(sandbox.path().join("advanced.work"))
                .unwrap()
                .trim(),
        )
        .unwrap();
    }
}
