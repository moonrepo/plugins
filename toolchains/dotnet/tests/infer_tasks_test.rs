use dotnet_toolchain::config::InferTasksSetting;
use dotnet_toolchain::infer_tasks::*;
use dotnet_toolchain::msbuild::MsbuildEvaluation;
use moon_common::Id;
use moon_config::{
    Input, Output, PartialTaskArgs, PartialTaskConfig, PartialTaskDependency,
    PartialTaskDependencyConfig, TaskOptionCache, TaskOptionRunInCI,
};
use moon_target::Target;
use std::collections::{BTreeMap, BTreeSet};

mod infer_tasks {
    use super::*;

    fn evaluation(properties: &[(&str, &str)]) -> MsbuildEvaluation {
        let mut evaluation = MsbuildEvaluation::default();

        for (name, value) in properties {
            evaluation
                .properties
                .insert(name.to_string(), value.to_string());
        }

        evaluation
    }

    fn infer(
        evaluation: &MsbuildEvaluation,
        setting: &InferTasksSetting,
        reserved: &[&str],
    ) -> BTreeMap<Id, PartialTaskConfig> {
        infer_tasks(
            setting,
            &reserved.iter().map(|id| id.to_string()).collect(),
            &InferInputs {
                evaluation,
                explicit_project_file: None,
                project_dir: "C:\\work\\repo\\app",
                workspace_dir: "C:\\work\\repo",
                test_platform_runner: false,
            },
        )
        .unwrap()
    }

    fn test_project_evaluation() -> MsbuildEvaluation {
        let mut eval = evaluation(&[("OutputType", "Exe"), ("TargetFramework", "net10.0")]);
        eval.items.insert(
            "PackageReference".into(),
            vec![serde_json::json!({ "Identity": "Microsoft.NET.Test.Sdk" })],
        );
        eval
    }

    /// An `Exe` with the given package references and no test-related property,
    /// mirroring what an unrestored tree reports.
    fn exe_with_packages(packages: &[&str]) -> MsbuildEvaluation {
        let mut eval = evaluation(&[("OutputType", "Exe"), ("TargetFramework", "net10.0")]);
        eval.items.insert(
            "PackageReference".into(),
            packages
                .iter()
                .map(|name| serde_json::json!({ "Identity": name }))
                .collect(),
        );
        eval
    }

    fn command_line(task: &PartialTaskConfig) -> String {
        match task.command.as_ref().unwrap() {
            PartialTaskArgs::List(list) => list.join(" "),
            PartialTaskArgs::String(value) => value.clone(),
            other => panic!("unexpected command shape: {other:?}"),
        }
    }

    #[test]
    fn classlib_gets_build_only() {
        let eval = evaluation(&[
            ("OutputType", "Library"),
            ("BaseOutputPath", "bin\\"),
            ("BaseIntermediateOutputPath", "obj\\"),
        ]);
        let tasks = infer(&eval, &InferTasksSetting::default(), &[]);

        assert_eq!(
            tasks.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
            vec!["build"]
        );

        let build = &tasks[&Id::raw("build")];
        assert_eq!(
            command_line(build),
            "dotnet build --no-restore --no-dependencies"
        );
        assert_eq!(
            build.outputs.as_ref().unwrap(),
            &vec![Output::parse("bin").unwrap()]
        );
        assert!(build.options.is_none(), "outputs known => cache untouched");
        assert!(build.deps.is_some());
        // Inputs exclude the evaluated output/intermediate dirs so hashes
        // stabilize (obj is mutated by every build).
        assert_eq!(
            build.inputs.as_ref().unwrap(),
            &vec![
                Input::parse("**/*").unwrap(),
                Input::parse("!bin/**").unwrap(),
                Input::parse("!obj/**").unwrap(),
            ]
        );
    }

    #[test]
    fn exe_gets_build_run_publish() {
        let eval = evaluation(&[
            ("OutputType", "Exe"),
            ("BaseOutputPath", "bin\\"),
            ("TargetFramework", "net8.0"),
            ("PublishDir", "bin\\Debug\\net8.0\\publish\\"),
        ]);
        let tasks = infer(&eval, &InferTasksSetting::default(), &[]);

        assert_eq!(
            tasks.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
            vec!["build", "publish", "run"]
        );

        let run = &tasks[&Id::raw("run")];
        assert_eq!(command_line(run), "dotnet run");
        let run_options = run.options.as_ref().unwrap();
        assert_eq!(run_options.cache, Some(TaskOptionCache::Enabled(false)));
        assert_eq!(
            run_options.run_in_ci,
            Some(TaskOptionRunInCI::Enabled(false))
        );

        let publish = &tasks[&Id::raw("publish")];
        assert_eq!(
            command_line(publish),
            "dotnet publish --no-build --no-restore"
        );
        assert_eq!(
            publish.outputs.as_ref().unwrap(),
            &vec![Output::parse("bin/Debug/net8.0/publish").unwrap()]
        );
    }

    #[test]
    fn test_project_gets_build_test_never_run() {
        // Modern test SDKs can flip OutputType to Exe — test wins over run.
        let mut eval = evaluation(&[("OutputType", "Exe"), ("TargetFramework", "net8.0")]);
        eval.items.insert(
            "PackageReference".into(),
            vec![serde_json::json!({ "Identity": "Microsoft.NET.Test.Sdk", "Version": "17.10.0" })],
        );

        let tasks = infer(&eval, &InferTasksSetting::default(), &[]);

        assert!(tasks.contains_key(&Id::raw("build")));
        assert!(tasks.contains_key(&Id::raw("test")));
        assert!(!tasks.contains_key(&Id::raw("run")));
        assert!(!tasks.contains_key(&Id::raw("publish")));

        let test = &tasks[&Id::raw("test")];
        assert_eq!(command_line(test), "dotnet test --no-build --no-restore");
    }

    #[test]
    fn pins_evaluated_configuration_on_cacheable_commands() {
        // `dotnet publish` defaults to Release (.NET 8+) while `dotnet build`
        // defaults to Debug — the explicit `-c` keeps `--no-build` coherent.
        let mut eval = evaluation(&[
            ("OutputType", "Exe"),
            ("TargetFramework", "net8.0"),
            ("Configuration", "Debug"),
            ("PublishDir", "bin\\Debug\\net8.0\\publish\\"),
        ]);
        eval.items.insert(
            "PackageReference".into(),
            vec![serde_json::json!({ "Identity": "Microsoft.NET.Test.Sdk" })],
        );

        let tasks = infer(&eval, &InferTasksSetting::default(), &[]);

        assert_eq!(
            command_line(&tasks[&Id::raw("build")]),
            "dotnet build --no-restore --no-dependencies -c Debug"
        );
        assert_eq!(
            command_line(&tasks[&Id::raw("test")]),
            "dotnet test --no-build --no-restore -c Debug"
        );
    }

    #[test]
    fn multi_tfm_exe_skips_publish() {
        let eval = evaluation(&[("OutputType", "Exe"), ("TargetFrameworks", "net8.0;net9.0")]);
        let tasks = infer(&eval, &InferTasksSetting::default(), &[]);

        assert!(tasks.contains_key(&Id::raw("run")));
        assert!(!tasks.contains_key(&Id::raw("publish")));
    }

    #[test]
    fn unknown_outputs_disable_caching_instead_of_guessing() {
        // BaseOutputPath redirected outside the workspace entirely.
        let eval = evaluation(&[
            ("OutputType", "Library"),
            ("BaseOutputPath", "D:\\global-outputs\\app\\"),
        ]);
        let tasks = infer(&eval, &InferTasksSetting::default(), &[]);

        let build = &tasks[&Id::raw("build")];
        assert!(build.outputs.is_none());
        assert_eq!(
            build.options.as_ref().unwrap().cache,
            Some(TaskOptionCache::Enabled(false))
        );
    }

    #[test]
    fn granular_selection_and_reserved_ids_are_respected() {
        let eval = evaluation(&[
            ("OutputType", "Exe"),
            ("BaseOutputPath", "bin\\"),
            ("TargetFramework", "net8.0"),
            ("PublishDir", "bin\\Debug\\net8.0\\publish\\"),
        ]);

        let only = InferTasksSetting::Only(vec!["run".into(), "publish".into()]);
        let tasks = infer(&eval, &only, &[]);
        assert_eq!(
            tasks.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
            vec!["publish", "run"],
            "granular selection"
        );

        let tasks = infer(&eval, &InferTasksSetting::default(), &["run", "build"]);
        assert_eq!(
            tasks.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
            vec!["publish"],
            "reserved (inherited) ids skipped"
        );

        let tasks = infer(&eval, &InferTasksSetting::Enabled(false), &[]);
        assert!(tasks.is_empty());
    }

    #[test]
    fn detects_test_projects_that_have_no_microsoft_net_test_sdk() {
        // Microsoft.Testing.Platform test projects replace Microsoft.NET.Test.Sdk
        // outright. Shapes taken from real repositories: dotnet/eShop uses
        // `<Project Sdk="MSTest.Sdk">`, which sets the property and references no
        // test package; OrchardCMS/OrchardCore uses `xunit.v3.mtp-v2` with
        // neither property set on an unrestored tree.
        let by_property = evaluation(&[
            ("OutputType", "Exe"),
            ("TargetFramework", "net10.0"),
            ("IsTestingPlatformApplication", "true"),
        ]);

        for (label, eval) in [
            ("MSTest.Sdk property", by_property),
            ("xunit.v3 package", exe_with_packages(&["xunit.v3.mtp-v2"])),
            (
                "platform package",
                exe_with_packages(&["Microsoft.Testing.Platform.MSBuild"]),
            ),
        ] {
            let tasks = infer(&eval, &InferTasksSetting::default(), &[]);
            let ids = tasks.keys().map(|id| id.as_str()).collect::<Vec<_>>();

            assert!(
                ids.contains(&"test"),
                "{label}: expected a test task, got {ids:?}"
            );
            // A test project is not an application, so it gets neither of these.
            assert!(!ids.contains(&"run"), "{label}: {ids:?}");
            assert!(!ids.contains(&"publish"), "{label}: {ids:?}");
        }
    }

    #[test]
    fn does_not_mistake_test_helper_packages_for_a_test_project() {
        // All three appear in real test-adjacent projects. Matching "test" as a
        // substring would wrongly flag every one of them, and a BenchmarkDotNet
        // project explicitly sets the properties to `false`.
        let eval = exe_with_packages(&[
            "Microsoft.AspNetCore.Mvc.Testing",
            "Microsoft.AspNetCore.TestHost",
            "BenchmarkDotNet",
        ]);

        let tasks = infer(&eval, &InferTasksSetting::default(), &[]);
        let ids = tasks.keys().map(|id| id.as_str()).collect::<Vec<_>>();

        assert!(!ids.contains(&"test"), "{ids:?}");
        assert!(
            ids.contains(&"run"),
            "an executable must still get run: {ids:?}"
        );

        let benchmarks = evaluation(&[
            ("OutputType", "Exe"),
            ("TargetFramework", "net10.0"),
            ("IsTestProject", "false"),
            ("IsTestingPlatformApplication", "false"),
        ]);
        let tasks = infer(&benchmarks, &InferTasksSetting::default(), &[]);

        assert!(!tasks.keys().any(|id| id.as_str() == "test"));
    }

    #[test]
    fn the_self_build_dep_is_optional() {
        // moon defaults `~:` deps to mandatory, so selecting only `test` or
        // only `publish` — no `build` task to depend on — would fail
        // project-graph construction with `UnknownDepTarget` if these were
        // plain targets.
        // A project is either a test project or an executable — `is_exe`
        // excludes `is_test` — so each dep needs its own evaluation.
        let cases = [
            ("test", evaluation(&[("IsTestProject", "true")])),
            (
                "publish",
                evaluation(&[
                    ("OutputType", "Exe"),
                    ("TargetFramework", "net8.0"),
                    ("PublishDir", "bin\\Debug\\net8.0\\publish\\"),
                ]),
            ),
        ];

        for (id, eval) in cases {
            let tasks = infer(&eval, &InferTasksSetting::Only(vec![id.into()]), &[]);

            assert_eq!(
                tasks[&Id::raw(id)].deps.as_deref(),
                Some(
                    &[PartialTaskDependency::Object(PartialTaskDependencyConfig {
                        target: Some(Target::parse("~:build").unwrap()),
                        optional: Some(true),
                        ..Default::default()
                    })][..]
                ),
                "`{id}` must depend on an optional `~:build`"
            );
        }
    }

    #[test]
    fn multiple_project_files_get_explicit_targets() {
        let eval = evaluation(&[
            ("OutputType", "Exe"),
            ("BaseOutputPath", "bin\\"),
            ("TargetFramework", "net8.0"),
        ]);

        let tasks = infer_tasks(
            &InferTasksSetting::default(),
            &BTreeSet::new(),
            &InferInputs {
                evaluation: &eval,
                explicit_project_file: Some("App.csproj"),
                project_dir: "/repo/app",
                workspace_dir: "/repo",
                test_platform_runner: false,
            },
        )
        .unwrap();

        assert_eq!(
            command_line(&tasks[&Id::raw("build")]),
            "dotnet build App.csproj --no-restore --no-dependencies"
        );
        assert_eq!(
            command_line(&tasks[&Id::raw("run")]),
            "dotnet run --project App.csproj"
        );
    }

    #[test]
    fn test_platform_takes_the_project_through_a_flag() {
        let eval = test_project_evaluation();

        let infer_with = |runner: bool, file: Option<&str>| {
            let tasks = infer_tasks(
                &InferTasksSetting::default(),
                &BTreeSet::new(),
                &InferInputs {
                    evaluation: &eval,
                    explicit_project_file: file,
                    project_dir: "/repo/app-tests",
                    workspace_dir: "/repo",
                    test_platform_runner: runner,
                },
            )
            .unwrap();

            command_line(&tasks[&Id::raw("test")])
        };

        // MTP rejects a positional project path...
        assert_eq!(
            infer_with(true, Some("App.Tests.csproj")),
            "dotnet test --project App.Tests.csproj --no-build --no-restore"
        );
        // ...while classic VSTest mode rejects `--project`.
        assert_eq!(
            infer_with(false, Some("App.Tests.csproj")),
            "dotnet test App.Tests.csproj --no-build --no-restore"
        );
        // With one project file in the directory neither flavour applies:
        // the command runs in the project directory with no path at all.
        assert_eq!(
            infer_with(true, None),
            "dotnet test --no-build --no-restore"
        );
        assert_eq!(
            infer_with(false, None),
            "dotnet test --no-build --no-restore"
        );
    }

    #[test]
    fn project_level_test_platform_opt_in_is_honored() {
        // A project can select MTP on its own, without a global.json.
        let mut eval = test_project_evaluation();
        eval.properties
            .insert("TestingPlatformDotnetTestSupport".into(), "true".into());

        let tasks = infer_tasks(
            &InferTasksSetting::default(),
            &BTreeSet::new(),
            &InferInputs {
                evaluation: &eval,
                explicit_project_file: Some("App.Tests.csproj"),
                project_dir: "/repo/app-tests",
                workspace_dir: "/repo",
                test_platform_runner: false,
            },
        )
        .unwrap();

        assert_eq!(
            command_line(&tasks[&Id::raw("test")]),
            "dotnet test --project App.Tests.csproj --no-build --no-restore"
        );
    }

    #[test]
    fn reports_only_conflicts_that_actually_suppress_inference() {
        let reserved: BTreeMap<String, String> = [
            ("build", "/workspace/.moon/tasks/dotnet.yml"),
            ("publish", "/workspace/.moon/tasks.yml"),
            // Not inferable, so its presence is unremarkable.
            ("lint", "/workspace/.moon/tasks/all.yml"),
        ]
        .iter()
        .map(|(id, file)| (id.to_string(), file.to_string()))
        .collect();

        assert_eq!(
            reportable_conflicts(&reserved, &InferTasksSetting::default()),
            vec![
                ("build", "/workspace/.moon/tasks/dotnet.yml"),
                ("publish", "/workspace/.moon/tasks.yml"),
            ]
        );

        // A task the user did not ask us to infer is not a conflict.
        assert_eq!(
            reportable_conflicts(&reserved, &InferTasksSetting::Only(vec!["publish".into()])),
            vec![("publish", "/workspace/.moon/tasks.yml")]
        );

        assert!(reportable_conflicts(&reserved, &InferTasksSetting::Enabled(false)).is_empty());
        assert!(reportable_conflicts(&BTreeMap::new(), &InferTasksSetting::default()).is_empty());
    }

    #[test]
    fn resolves_output_paths_in_every_form() {
        // Relative stays relative.
        assert_eq!(
            resolve_output_path("bin\\", "C:\\repo\\app", "C:\\repo"),
            Some("bin".into())
        );
        // Absolute under the project dir, case-insensitive.
        assert_eq!(
            resolve_output_path("C:\\Repo\\App\\bin\\Debug\\", "c:\\repo\\app", "c:\\repo"),
            Some("bin/Debug".into())
        );
        // Absolute under the workspace (artifacts layout) => workspace-relative.
        assert_eq!(
            resolve_output_path(
                "C:\\repo\\artifacts\\bin\\app\\",
                "C:\\repo\\app",
                "C:\\repo"
            ),
            Some("/artifacts/bin/app".into())
        );
        // Unix forms.
        assert_eq!(
            resolve_output_path("/repo/app/bin", "/repo/app", "/repo"),
            Some("bin".into())
        );
        // Outside the workspace => not resolvable.
        assert_eq!(
            resolve_output_path("D:\\elsewhere\\bin", "C:\\repo\\app", "C:\\repo"),
            None
        );
        // Empty => not resolvable.
        assert_eq!(resolve_output_path("", "C:\\repo\\app", "C:\\repo"), None);
        // Prefix must respect component boundaries.
        assert_eq!(
            resolve_output_path("/repo/app-other/bin", "/repo/app", "/repo"),
            Some("/app-other/bin".into())
        );
    }
}
