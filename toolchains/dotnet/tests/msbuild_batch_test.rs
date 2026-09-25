//! Requires a .NET SDK (8+) on `PATH`: these tests spawn a real
//! `dotnet msbuild` outside the wasm plugin, which is the point of the file —
//! the plugin's per-project fallback would silently mask a broken batch in the
//! sandbox tests. `-getItem` JSON output is what needs SDK 8+.

use dotnet_toolchain::msbuild::{
    MsbuildEvaluation, detect_failed_projects, moon_eval_targets_xml, normalize_path_key,
    parse_batch_output, traversal_project_xml,
};
use starbase_sandbox::create_empty_sandbox;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod msbuild {
    use super::*;

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/__fixtures__/projects")
    }

    /// Stage the generated traversal project alongside the injected targets file
    /// and evaluate it, exactly as `evaluate_projects_batch` does.
    fn run_batch(scratch: &Path, projects: &[String]) -> Output {
        std::fs::write(scratch.join("moon-eval.targets"), moon_eval_targets_xml()).unwrap();
        std::fs::write(
            scratch.join("traversal.proj"),
            traversal_project_xml(projects),
        )
        .unwrap();

        Command::new("dotnet")
            .args([
                "msbuild",
                scratch.join("traversal.proj").to_str().unwrap(),
                "-nologo",
                "-maxCpuCount",
                "-nodeReuse:false",
                "-t:MoonCollect",
                "-getItem:MoonEval",
            ])
            .output()
            .expect("failed to spawn `dotnet msbuild`")
    }

    /// End-to-end validation of the batched evaluation mechanism against a real
    /// MSBuild.
    #[test]
    fn batched_traversal_evaluates_fixture_projects() {
        let sandbox = create_empty_sandbox();

        let projects = [
            ["app", "App.csproj"],
            ["lib", "Lib.csproj"],
            ["core", "Core.csproj"],
            ["app-tests", "App.Tests.csproj"],
        ]
        .iter()
        .map(|[dir, file]| {
            fixtures()
                .join(dir)
                .join(file)
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<_>>();

        let output = run_batch(sandbox.path(), &projects);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "batch invocation failed ({:?}):\n{stdout}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
        );

        let results = parse_batch_output(&stdout).unwrap();

        let get = |index: usize| -> &MsbuildEvaluation {
            let key = normalize_path_key(&projects[index]);

            results
                .get(&key)
                .unwrap_or_else(|| panic!("missing {key} in batch output: {:?}", results.keys()))
        };

        // app -> lib, and its Exe OutputType survives the round-trip.
        let app = get(0);
        let app_refs = app.project_reference_paths();
        assert_eq!(app_refs.len(), 1);
        assert!(normalize_path_key(&app_refs[0]).ends_with("lib/lib.csproj"));
        assert_eq!(app.property("OutputType"), "Exe");

        // lib -> core.
        let lib_refs = get(1).project_reference_paths();
        assert_eq!(lib_refs.len(), 1);
        assert!(normalize_path_key(&lib_refs[0]).ends_with("core/core.csproj"));

        // core has no references at all.
        let core = get(2);
        assert!(core.project_reference_paths().is_empty());
        assert!(core.package_references().is_empty());

        // app-tests -> app, with its evaluated package set intact.
        let tests = get(3);
        let tests_refs = tests.project_reference_paths();
        assert!(normalize_path_key(&tests_refs[0]).ends_with("app/app.csproj"));

        let packages = tests.package_references();
        assert_eq!(packages.get("Microsoft.NET.Test.Sdk").unwrap(), "17.10.0");
        assert_eq!(packages.get("xunit").unwrap(), "2.8.0");
    }

    /// Documents the MSBuild behavior the retry logic in
    /// `evaluate_projects_batch` exists for: one unloadable project makes the
    /// whole batch return exit != 0 with ZERO target outputs (`ContinueOnError`
    /// does not rescue load errors) — and validates that
    /// `detect_failed_projects` identifies exactly the offender from the real
    /// error output, so the retry can exclude it.
    #[test]
    fn broken_project_aborts_batch_and_is_detectable() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(
            "broken/Broken.csproj",
            "<Project Sdk=\"Microsoft.NET.Sdk\"><broken",
        );

        let projects = vec![
            fixtures()
                .join("core")
                .join("Core.csproj")
                .to_string_lossy()
                .to_string(),
            sandbox
                .path()
                .join("broken/Broken.csproj")
                .to_string_lossy()
                .to_string(),
        ];

        let output = run_batch(sandbox.path(), &projects);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // The whole batch fails — not just the broken project.
        assert!(!output.status.success());
        assert!(parse_batch_output(&stdout).unwrap().is_empty());

        // But the offender is identifiable from the error lines, and the healthy
        // project is not falsely implicated.
        let failed = detect_failed_projects(&format!("{stdout}{stderr}"), &projects);
        assert_eq!(failed, vec![projects[1].clone()]);
    }
}
