use moon_pdk_api::*;
use moon_pdk_test_utils::create_empty_moon_sandbox;
use serde_json::json;

/// Number of generated projects. Above the ~50 the follow-up called for, and
/// enough that per-project MSBuild startup would dominate if batching broke.
const PROJECTS: usize = 60;

/// Fan-out of the generated reference graph: project N references N-1 and
/// N-FANOUT, giving every project multiple inbound and outbound edges
/// instead of one long chain.
const FANOUT: usize = 7;

/// Soak test: generate a large workspace at runtime and run the real project
/// graph extension over it. Ignored by default — it shells out to a real
/// MSBuild evaluation over 60 projects, which is minutes of work if the
/// batched path regresses to per-project evaluation.
///
/// Run with:
///   cargo nextest run -p dotnet_toolchain --no-default-features ///     --run-ignored=only soak_project_graph_at_scale
#[tokio::test(flavor = "multi_thread")]
#[ignore = "generates and evaluates a 60-project workspace"]
async fn soak_project_graph_at_scale() {
    let sandbox = create_empty_moon_sandbox();

    let project_id = |index: usize| format!("proj{index:03}");

    let mut sources = String::from("projects:\n");

    for index in 0..PROJECTS {
        let id = project_id(index);

        sources.push_str(&format!("  - '{id}'\n"));

        // Reference the immediately previous project and one FANOUT back, so
        // the graph has depth and breadth without cycles.
        let references = [index.checked_sub(1), index.checked_sub(FANOUT)]
            .into_iter()
            .flatten()
            .map(|dep| {
                let dep_id = project_id(dep);

                format!("    <ProjectReference Include=\"..\\{dep_id}\\{dep_id}.csproj\" />\n")
            })
            .collect::<String>();

        sandbox.create_file(
            format!("{id}/{id}.csproj"),
            format!(
                r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
  <ItemGroup>
{references}  </ItemGroup>
</Project>
"#
            ),
        );

        sandbox.create_file(
            format!("{id}/Class1.cs"),
            format!("namespace {id};\n\npublic class Class1;\n"),
        );

        sandbox.create_file(
            format!("{id}/moon.yml"),
            "language: 'csharp'\n\ntoolchains:\n  default: 'dotnet'\n",
        );
    }

    sandbox.create_file(".moon/workspace.yml", &sources);

    let plugin = sandbox.create_toolchain("dotnet").await;

    let mut input = ExtendProjectGraphInput::default();

    for index in 0..PROJECTS {
        input
            .project_sources
            .insert(Id::raw(project_id(index)), project_id(index));
    }

    input.toolchain_config = json!({ "inferDependencies": true });

    let started = std::time::Instant::now();
    let output = plugin.extend_project_graph(input).await;
    let elapsed = started.elapsed();

    // Every project except the first contributes its dependencies; the
    // first has none, so it appears only for its alias.
    assert_eq!(output.extended_projects.len(), PROJECTS);
    assert_eq!(output.input_files.len(), PROJECTS);

    for index in 0..PROJECTS {
        let extended = &output.extended_projects[&Id::raw(project_id(index))];

        let expected = [index.checked_sub(1), index.checked_sub(FANOUT)]
            .into_iter()
            .flatten()
            .map(project_id)
            .collect::<std::collections::BTreeSet<_>>();

        let actual = extended
            .dependencies
            .iter()
            .map(|dep| dep.id.to_string())
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(actual, expected, "dependencies for {}", project_id(index));
    }

    // Informational: run with --nocapture to see the timing. Batched
    // evaluation lands in seconds; a regression to per-project evaluation
    // costs ~0.5s per project (minutes at this size).
    println!(
        "soak: {PROJECTS} projects evaluated in {:.1}s ({:.0}ms/project)",
        elapsed.as_secs_f64(),
        elapsed.as_millis() as f64 / PROJECTS as f64,
    );
}
