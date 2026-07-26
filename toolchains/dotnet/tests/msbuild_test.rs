use dotnet_toolchain::msbuild::*;

mod msbuild {
    use super::*;

    // Abridged real output shape from `dotnet msbuild -getProperty:... -getItem:...`
    // on .NET SDK 8+.
    const SAMPLE: &str = r#"{
  "Properties": {
    "TargetFramework": "",
    "TargetFrameworks": "net8.0;net9.0",
    "OutputType": "Exe",
    "IsTestProject": "",
    "IsPackable": "true",
    "RestorePackagesWithLockFile": ""
  },
  "Items": {
    "ProjectReference": [
      {
        "Identity": "..\\LibA\\LibA.csproj",
        "FullPath": "C:\\abs\\path\\LibA\\LibA.csproj",
        "Filename": "LibA",
        "Extension": ".csproj",
        "DefiningProjectFullPath": "C:\\abs\\path\\App\\App.csproj"
      }
    ],
    "PackageReference": [
      { "Identity": "Newtonsoft.Json", "Version": "13.0.3" },
      { "Identity": "NoVersionPkg" }
    ],
    "PackageVersion": [
      { "Identity": "Newtonsoft.Json", "Version": "13.0.3" }
    ]
  }
}"#;

    #[test]
    fn parses_real_output_shape() {
        let eval = parse_msbuild_output(SAMPLE).unwrap();

        assert_eq!(eval.property("OutputType"), "Exe");
        assert_eq!(eval.property("TargetFrameworks"), "net8.0;net9.0");
        assert_eq!(eval.property("TargetFramework"), "");
        assert_eq!(eval.property("IsPackable"), "true");
        assert_eq!(
            eval.project_reference_paths(),
            vec!["C:\\abs\\path\\LibA\\LibA.csproj".to_string()]
        );

        let packages = eval.package_references();
        assert_eq!(packages.get("Newtonsoft.Json").unwrap(), "13.0.3");
        assert_eq!(packages.get("NoVersionPkg").unwrap(), "*");

        let versions = eval.package_versions();
        assert_eq!(versions.get("Newtonsoft.Json").unwrap(), "13.0.3");
    }

    #[test]
    fn skips_leading_noise_before_json() {
        let noisy = format!("some warning: blah\nanother line\n{SAMPLE}");
        let eval = parse_msbuild_output(&noisy).unwrap();

        assert_eq!(eval.property("OutputType"), "Exe");
    }

    #[test]
    fn errors_when_no_json() {
        assert!(parse_msbuild_output("MSBUILD : error MSB1063: ...").is_err());
    }

    #[test]
    fn empty_items_and_properties() {
        let eval = parse_msbuild_output("{}").unwrap();

        assert_eq!(eval.property("OutputType"), "");
        assert!(eval.project_reference_paths().is_empty());
        assert!(eval.package_references().is_empty());
    }

    // Abridged real output shape from a batched traversal invocation
    // (`-t:MoonCollect -getItem:MoonEval`), including MSBuild's well-known
    // metadata noise and a Windows 8.3 short path in OriginalItemSpec.
    const BATCH_SAMPLE: &str = r#"{
  "Items": {
    "MoonEval": [
      {
        "Identity": "C:\\long\\path\\app\\App.csproj",
        "MSBuildSourceProjectFile": "C:\\long\\path\\app\\App.csproj",
        "MSBuildSourceTargetName": "MoonEvalInner",
        "OriginalItemSpec": "C:\\LONGPA~1\\app\\App.csproj",
        "OutputType": "Exe",
        "TargetFramework": "net8.0",
        "TargetFrameworks": "",
        "IsTestProject": "",
        "IsPackable": "",
        "RestorePackagesWithLockFile": "",
        "MoonProjectRefs": "C:\\long\\path\\lib\\Lib.csproj|C:\\long\\path\\core\\Core.csproj",
        "MoonPackageRefs": "Newtonsoft.Json@13.0.3|CpmPackage@",
        "Filename": "App",
        "Extension": ".csproj"
      },
      {
        "Identity": "C:\\long\\path\\multi\\Multi.csproj",
        "MSBuildSourceProjectFile": "C:\\long\\path\\multi\\Multi.csproj",
        "OriginalItemSpec": "C:\\long\\path\\multi\\Multi.csproj",
        "OutputType": "Library",
        "TargetFramework": "",
        "TargetFrameworks": "net8.0;netstandard2.0",
        "IsTestProject": "",
        "IsPackable": "",
        "RestorePackagesWithLockFile": "",
        "MoonProjectRefs": "",
        "MoonPackageRefs": ""
      }
    ]
  }
}"#;

    #[test]
    fn parses_batch_output_per_project() {
        let results = parse_batch_output(BATCH_SAMPLE).unwrap();

        // Keyed by both the expanded path and the 8.3 short form we passed.
        let app = &results["c:/long/path/app/app.csproj"];
        assert!(results.contains_key("c:/longpa~1/app/app.csproj"));

        assert_eq!(app.property("OutputType"), "Exe");
        assert_eq!(app.property("TargetFramework"), "net8.0");
        assert_eq!(
            app.project_reference_paths(),
            vec![
                "C:\\long\\path\\lib\\Lib.csproj".to_string(),
                "C:\\long\\path\\core\\Core.csproj".to_string(),
            ]
        );

        let packages = app.package_references();
        assert_eq!(packages.get("Newtonsoft.Json").unwrap(), "13.0.3");
        // Versionless (CPM-style) entries fall back to `*`.
        assert_eq!(packages.get("CpmPackage").unwrap(), "*");

        let multi = &results["c:/long/path/multi/multi.csproj"];
        assert_eq!(multi.property("TargetFrameworks"), "net8.0;netstandard2.0");
        assert!(multi.project_reference_paths().is_empty());
        assert!(multi.package_references().is_empty());
    }

    #[test]
    fn batch_output_without_items_is_empty() {
        assert!(parse_batch_output("{}").unwrap().is_empty());
    }

    #[test]
    fn finds_the_common_source_prefix() {
        // The common shape in a polyglot repository: every .NET project sits
        // under one subtree, so a `global.json` there governs evaluation just as
        // it governs the tasks that run inside it.
        assert_eq!(
            common_source_prefix(&[
                "src/backend/Attachment/Attachment.Service",
                "src/backend/Common/Common.Business",
                "src/backend/Billing/Billing.Service",
            ]),
            "src/backend"
        );

        // Mixed trees share nothing -> the workspace root.
        assert_eq!(
            common_source_prefix(&["src/backend/App", "tools/Generator"]),
            ""
        );

        // A single project yields its own directory; separators normalize.
        assert_eq!(common_source_prefix(&["apps\\api"]), "apps/api");
        assert_eq!(common_source_prefix(&["."]), "");
        assert_eq!(common_source_prefix(&[]), "");
        // No accidental partial-component matches.
        assert_eq!(common_source_prefix(&["src/app", "src/app-other"]), "src");
    }

    #[test]
    fn escapes_msbuild_includes() {
        assert_eq!(
            escape_msbuild_include("C:\\repo\\A & B\\$(odd)@*?;<x>\"100%\".csproj"),
            "C:\\repo\\A &amp; B\\%24(odd)%40%2A%3F%3B&lt;x&gt;&quot;100%25&quot;.csproj"
        );
    }

    #[test]
    fn targets_xml_covers_all_eval_properties() {
        let xml = moon_eval_targets_xml();

        for prop in EVAL_PROPERTIES.split(',') {
            assert!(
                xml.contains(&format!("<{prop}>$({prop})</{prop}>")),
                "{prop}"
            );
        }

        assert!(xml.contains("MoonProjectRefs"));
        assert!(xml.contains("MoonPackageRefs"));
    }

    #[test]
    fn traversal_xml_lists_projects_and_injects_both_hooks() {
        let xml = traversal_project_xml(&[
            "C:\\repo\\a\\a.csproj".to_string(),
            "/home/x/b & c/b.csproj".to_string(),
        ]);

        assert!(xml.contains("Include=\"C:\\repo\\a\\a.csproj\""));
        assert!(xml.contains("Include=\"/home/x/b &amp; c/b.csproj\""));
        // Both hooks: plain SDK projects import CustomAfterMicrosoftCommonTargets,
        // multi-TFM outer builds import the CrossTargeting variant instead.
        assert!(xml.contains(
            "CustomAfterMicrosoftCommonTargets=$(MSBuildThisFileDirectory)moon-eval.targets"
        ));
        assert!(xml.contains("CustomAfterMicrosoftCommonCrossTargetingTargets=$(MSBuildThisFileDirectory)moon-eval.targets"));
        assert!(xml.contains("BuildInParallel=\"true\""));
        assert!(xml.contains("ContinueOnError=\"WarnAndContinue\""));
    }

    #[test]
    fn recognizes_sdk_resolution_failures() {
        // Real host output (abridged) from a workspace pinning an SDK that
        // is not installed.
        let output = "\
5.0.100 [C:\\Program Files\\dotnet\\sdk]
      A compatible .NET SDK was not found.

Requested SDK version: 10.0.301
global.json file: C:\\repo\\src\\backend\\global.json

Learn about SDK resolution:
https://aka.ms/dotnet/sdk-not-found";

        assert!(is_sdk_resolution_failure(output));
        // The URL alone is enough, so localized message text still matches.
        assert!(is_sdk_resolution_failure(
            "irgendein Fehler\nhttps://aka.ms/dotnet/sdk-not-found"
        ));

        // A broken project is a different failure class and must not be
        // reported as a missing SDK.
        assert!(!is_sdk_resolution_failure(
            "C:\\repo\\app\\App.csproj(1,41): error MSB4025: The project file could not be loaded."
        ));
        assert!(!is_sdk_resolution_failure(
            "error MSB1009: Project file does not exist."
        ));
    }

    #[test]
    fn detects_failed_projects_across_short_and_long_path_forms() {
        // Real shape from GitHub's windows-latest runners: we pass a path
        // with an 8.3 short-name prefix (from %TEMP%), MSBuild's error line
        // prints the expanded long form.
        let output = "C:\\Users\\runneradmin\\AppData\\Local\\Temp\\scratch\\broken\\Broken.csproj(1,41): error MSB4025: The project file could not be loaded.";

        let paths = vec![
            "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\scratch\\broken/Broken.csproj".to_string(),
            "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\scratch\\ok\\Ok.csproj".to_string(),
        ];

        assert_eq!(
            detect_failed_projects(output, &paths),
            vec![paths[0].clone()]
        );
        assert!(detect_failed_projects("no errors here", &paths).is_empty());
    }

    #[test]
    fn detects_failed_projects_without_a_line_and_column() {
        // Verbatim shape from SDK 10.0.201 for an unresolvable SDK reference:
        // no line/column, and no error code either. Matching only the
        // `<path>(line,col):` form left the batch with no offender to exclude,
        // so the retry never happened.
        let output = "C:\\ws\\bad\\BadSdk.csproj : error : Could not resolve SDK \"Totally.Bogus.Sdk\". Exactly one of the probing messages below indicates why we could not resolve the SDK.";

        let paths = vec![
            "C:\\ws\\bad\\BadSdk.csproj".to_string(),
            "C:\\ws\\ok\\Ok.csproj".to_string(),
        ];

        assert_eq!(
            detect_failed_projects(output, &paths),
            vec![paths[0].clone()]
        );
    }

    #[test]
    fn normalizes_path_keys() {
        assert_eq!(
            normalize_path_key("C:\\Abs\\Path\\LibA\\LibA.csproj"),
            "c:/abs/path/liba/liba.csproj"
        );
        assert_eq!(
            normalize_path_key("/home/x/App.csproj"),
            "/home/x/app.csproj"
        );
    }
}
