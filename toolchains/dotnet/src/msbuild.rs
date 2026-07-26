use moon_pdk_api::AnyResult;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Result of an MSBuild evaluation via `dotnet msbuild -getProperty:... -getItem:...`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct MsbuildEvaluation {
    #[serde(rename = "Properties", default)]
    pub properties: BTreeMap<String, String>,

    #[serde(rename = "Items", default)]
    pub items: BTreeMap<String, Vec<serde_json::Value>>,
}

impl MsbuildEvaluation {
    pub fn property(&self, name: &str) -> &str {
        self.properties.get(name).map(String::as_str).unwrap_or("")
    }

    /// `FullPath` of every ProjectReference item (host-real absolute paths).
    pub fn project_reference_paths(&self) -> Vec<String> {
        self.items
            .get("ProjectReference")
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("FullPath"))
                    .filter_map(|value| value.as_str())
                    .map(|value| value.to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// PackageReference `Identity` -> `Version` (missing version becomes `*`).
    pub fn package_references(&self) -> BTreeMap<String, String> {
        self.identity_version_items("PackageReference", "*")
    }

    /// PackageVersion `Identity` -> `Version` (Central Package Management
    /// declarations from `Directory.Packages.props`; empty without CPM).
    pub fn package_versions(&self) -> BTreeMap<String, String> {
        self.identity_version_items("PackageVersion", "")
    }

    fn identity_version_items(
        &self,
        item_type: &str,
        missing_version: &str,
    ) -> BTreeMap<String, String> {
        self.items
            .get(item_type)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let identity = item.get("Identity")?.as_str()?;
                        let version = item
                            .get("Version")
                            .and_then(|value| value.as_str())
                            .filter(|value| !value.is_empty())
                            .unwrap_or(missing_version);

                        Some((identity.to_owned(), version.to_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The exact `-getProperty` list requested per evaluation.
/// `BaseOutputPath`/`BaseIntermediateOutputPath`/`PublishDir` feed inferred
/// task outputs and input exclusions (they follow redirected output
/// locations, e.g. .NET 8 `UseArtifactsOutput`). `AssemblyName`/`Version`
/// feed the project alias and manifest metadata.
/// `TestingPlatformDotnetTestSupport` opts a single project into
/// Microsoft.Testing.Platform, which changes the `dotnet test` command line.
/// `IsTestingPlatformApplication` identifies a test project that carries no
/// `Microsoft.NET.Test.Sdk` reference at all, which is the norm for the
/// test-oriented project SDKs (`<Project Sdk="MSTest.Sdk">`).
pub const EVAL_PROPERTIES: &str = "TargetFramework,TargetFrameworks,OutputType,IsTestProject,IsTestingPlatformApplication,IsPackable,RestorePackagesWithLockFile,BaseOutputPath,BaseIntermediateOutputPath,PublishDir,Configuration,AssemblyName,Version,TestingPlatformDotnetTestSupport";

/// The exact `-getItem` list requested per evaluation. `PackageVersion`
/// items exist under Central Package Management (declared in
/// `Directory.Packages.props`) and are empty otherwise.
///
/// Only `ProjectReference` and `PackageReference` come back from *batched*
/// evaluation — the injected target flattens those two into item metadata.
/// `PackageVersion` is therefore only ever populated on the per-project path,
/// which is where it is needed: `parse_manifest` evaluates a
/// `Directory.Packages.props` singly.
pub const EVAL_ITEMS: &str = "ProjectReference,PackageReference,PackageVersion";

/// Parse the stdout of an MSBuild `-get*` invocation. MSBuild may print stray
/// warnings before the JSON — start at the first `{`.
pub fn parse_msbuild_output(stdout: &str) -> AnyResult<MsbuildEvaluation> {
    let json_start = stdout
        .find('{')
        .ok_or_else(|| moon_pdk_api::anyhow!("no JSON found in MSBuild output"))?;

    Ok(serde_json::from_str(&stdout[json_start..])?)
}

/// Lexically normalize a host path for cross-referencing MSBuild output
/// against moon project paths: forward slashes, lowercased (paths on Windows
/// are case-insensitive; MSBuild output casing is not guaranteed to match
/// the on-disk casing moon reports).
pub fn normalize_path_key(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// Host environment applied to every MSBuild invocation, so graph evaluation
/// resolves the same SDK that tasks will run under.
#[derive(Clone, Debug, Default)]
pub struct EvalEnv {
    /// `DOTNET_ROOT` to evaluate under, when one was resolved.
    pub dotnet_root: Option<String>,

    /// Absolute path to the `dotnet` muxer inside `dotnet_root`, when its
    /// existence could be confirmed.
    ///
    /// This is what actually selects the SDK: the host resolves a bare
    /// command name from its own `PATH` (warpgate `host.rs` — a command
    /// containing a separator is treated as a path, anything else goes
    /// through `find_command_on_path`), and the muxer locates SDKs relative
    /// to its own location rather than from `DOTNET_ROOT`. Verified
    /// empirically: setting only `DOTNET_ROOT`/`paths` left evaluation on the
    /// `PATH` SDK.
    pub dotnet_exe: Option<String>,

    /// Directory to run MSBuild in. The dotnet host resolves `global.json`
    /// from the **current directory** — not from the project path (verified
    /// empirically) — so this is what decides which SDK evaluates the
    /// projects. Leaving it unset would inherit moon's own working directory,
    /// making evaluation depend on where the user happened to run moon from.
    pub cwd: Option<moon_pdk_api::VirtualPath>,
}

/// Deepest directory that contains all of the given workspace-relative
/// project sources, as a workspace-relative path (empty when they share no
/// prefix, i.e. the workspace root itself).
///
/// Used as the evaluation working directory: in a repo whose .NET projects
/// live under one subtree, this is the subtree root, so a `global.json` there
/// applies to evaluation exactly as it applies to the tasks that run inside
/// it.
pub fn common_source_prefix(sources: &[&str]) -> String {
    let mut common: Option<Vec<&str>> = None;

    for source in sources {
        let parts = source
            .split(['/', '\\'])
            .filter(|part| !part.is_empty() && *part != ".")
            .collect::<Vec<_>>();

        common = Some(match common {
            None => parts,
            Some(existing) => existing
                .into_iter()
                .zip(parts)
                // Sources come from moon config, which is case-sensitive
                // about the paths it reports; compare them verbatim.
                .take_while(|(left, right)| left == right)
                .map(|(left, _)| left)
                .collect(),
        });

        if common.as_ref().is_some_and(|parts| parts.is_empty()) {
            break;
        }
    }

    common.unwrap_or_default().join("/")
}

/// Escape a literal path for use inside an MSBuild `Include` attribute:
/// MSBuild's own special characters (property/item expansion, list
/// separators, globs) via `%XX` escapes, then XML attribute characters.
pub fn escape_msbuild_include(path: &str) -> String {
    path.replace('%', "%25")
        .replace('$', "%24")
        .replace('@', "%40")
        .replace(';', "%3B")
        .replace('*', "%2A")
        .replace('?', "%3F")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Item metadata name carrying the `|`-joined ProjectReference full paths in
/// batched evaluation output.
const BATCH_PROJECT_REFS: &str = "MoonProjectRefs";

/// Item metadata name carrying the `|`-joined `Identity@Version`
/// PackageReference entries in batched evaluation output.
const BATCH_PACKAGE_REFS: &str = "MoonPackageRefs";

/// The `.targets` file injected into every project during batched evaluation
/// (via the `CustomAfterMicrosoftCommon(CrossTargeting)Targets` hooks): a
/// target that returns the project's evaluation state as one item with
/// metadata. It runs as the entry target with no dependencies, so item state
/// when it executes is identical to evaluation state — the same answers as a
/// per-project `-getItem` query.
pub fn moon_eval_targets_xml() -> String {
    let properties = EVAL_PROPERTIES
        .split(',')
        .map(|prop| format!("        <{prop}>$({prop})</{prop}>\n"))
        .collect::<String>();

    format!(
        r#"<Project>
  <Target Name="MoonEvalInner" Returns="@(_MoonEvalResult)">
    <ItemGroup>
      <_MoonEvalResult Include="$(MSBuildProjectFullPath)">
{properties}        <{BATCH_PROJECT_REFS}>@(ProjectReference->'%(FullPath)', '|')</{BATCH_PROJECT_REFS}>
        <{BATCH_PACKAGE_REFS}>@(PackageReference->'%(Identity)@%(Version)', '|')</{BATCH_PACKAGE_REFS}>
      </_MoonEvalResult>
    </ItemGroup>
  </Target>
</Project>
"#
    )
}

/// The traversal project for batched evaluation: fans out to every listed
/// project with `BuildInParallel` (in-process MSBuild worker nodes) and
/// collects the injected target's outputs. A raw `<Project>` with no `Sdk`
/// attribute imports nothing implicitly, so the workspace's own
/// `Directory.Build.props` cannot interfere with the traversal itself, while
/// the child projects still evaluate with their full normal import chains.
/// `ContinueOnError` keeps one broken project from aborting the batch — it
/// just goes missing from the output (and falls back to per-project
/// evaluation).
pub fn traversal_project_xml(project_paths: &[String]) -> String {
    let includes = project_paths
        .iter()
        .map(|path| {
            format!(
                "    <MoonProject Include=\"{}\" />\n",
                escape_msbuild_include(path)
            )
        })
        .collect::<String>();

    format!(
        r#"<Project DefaultTargets="MoonCollect">
  <ItemGroup>
{includes}  </ItemGroup>
  <Target Name="MoonCollect" Returns="@(MoonEval)">
    <MSBuild
      Projects="@(MoonProject)"
      Targets="MoonEvalInner"
      BuildInParallel="true"
      ContinueOnError="WarnAndContinue"
      Properties="CustomAfterMicrosoftCommonTargets=$(MSBuildThisFileDirectory)moon-eval.targets;CustomAfterMicrosoftCommonCrossTargetingTargets=$(MSBuildThisFileDirectory)moon-eval.targets">
      <Output TaskParameter="TargetOutputs" ItemName="MoonEval" />
    </MSBuild>
  </Target>
</Project>
"#
    )
}

/// Parse the `-getItem:MoonEval` JSON of a batched traversal invocation into
/// per-project evaluations. Each project is keyed (normalized) by every
/// identifying path on its item: the traversal `Include` we wrote
/// (`OriginalItemSpec`) and MSBuild's own expanded full path
/// (`MSBuildSourceProjectFile` / `Identity`) — these can differ lexically,
/// e.g. Windows 8.3 short names in temp directories.
pub fn parse_batch_output(stdout: &str) -> AnyResult<BTreeMap<String, MsbuildEvaluation>> {
    let raw = parse_msbuild_output(stdout)?;
    let mut results = BTreeMap::new();

    let Some(items) = raw.items.get("MoonEval") else {
        return Ok(results);
    };

    for item in items {
        let metadata = |name: &str| {
            item.get(name)
                .and_then(|value| value.as_str())
                .unwrap_or("")
        };

        let mut evaluation = MsbuildEvaluation::default();

        for prop in EVAL_PROPERTIES.split(',') {
            evaluation
                .properties
                .insert(prop.to_owned(), metadata(prop).to_owned());
        }

        let project_refs = metadata(BATCH_PROJECT_REFS)
            .split('|')
            .filter(|path| !path.is_empty())
            .map(|path| serde_json::json!({ "FullPath": path }))
            .collect::<Vec<_>>();

        if !project_refs.is_empty() {
            evaluation
                .items
                .insert("ProjectReference".to_owned(), project_refs);
        }

        let package_refs = metadata(BATCH_PACKAGE_REFS)
            .split('|')
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                // '@' cannot appear in NuGet package ids or versions; an
                // empty version (e.g. Central Package Management) becomes
                // `*` downstream.
                let (identity, version) = entry.rsplit_once('@').unwrap_or((entry, ""));
                serde_json::json!({ "Identity": identity, "Version": version })
            })
            .collect::<Vec<_>>();

        if !package_refs.is_empty() {
            evaluation
                .items
                .insert("PackageReference".to_owned(), package_refs);
        }

        for key_field in ["OriginalItemSpec", "MSBuildSourceProjectFile", "Identity"] {
            let key = metadata(key_field);

            if !key.is_empty() {
                results.insert(normalize_path_key(key), evaluation.clone());
            }
        }
    }

    Ok(results)
}

/// Did an invocation fail because the dotnet host could not resolve an SDK
/// (rather than because a project is broken)?
///
/// Matches on the help URL the host prints, which — unlike the surrounding
/// message text — is not localized. The English phrasing is accepted as a
/// fallback for hosts that omit the link.
pub fn is_sdk_resolution_failure(output: &str) -> bool {
    let lower = output.to_lowercase();

    lower.contains("aka.ms/dotnet/sdk-not-found")
        || (lower.contains("global.json") && lower.contains("sdk") && lower.contains("not found"))
}

/// Given the output of a failed batch invocation, find which of the input
/// projects MSBuild reported diagnostics for.
///
/// MSBuild writes the file in two shapes, and both have to be recognized:
///
/// ```text
/// <path>(6,3): error MSB4025: The project file could not be loaded. ...
/// <path> : error : Could not resolve SDK "Totally.Bogus.Sdk". ...
/// ```
///
/// The second form — no line/column, emitted for SDK resolution among others —
/// was missed, so a batch killed by an unresolvable SDK reference identified no
/// offender, the retry below never fired, and the whole batch was discarded.
///
/// Only the trailing `<parent>/<file>` suffix is matched, not the full path:
/// MSBuild prints expanded long paths, which can differ lexically from the ones
/// we passed (e.g. Windows 8.3 short names like `RUNNER~1` in a temp-dir
/// prefix). Both anchors keep the match at a token boundary so `App.csproj`
/// cannot match `MyApp.csproj`.
///
/// Deliberately not filtered to lines containing `": error "`: MSBuild localizes
/// diagnostic text, so that would break on a non-English host. The cost is that
/// a path mentioned in a *warning* is treated as failed too — which merely
/// over-excludes, and an over-excluded project falls back to per-project
/// evaluation and stays correct. The same is true of a suffix shared by two
/// projects.
pub fn detect_failed_projects(output: &str, project_paths: &[String]) -> Vec<String> {
    let haystack = normalize_path_key(output);

    project_paths
        .iter()
        .filter(|path| {
            let normalized = normalize_path_key(path);

            // From the second-to-last separator: "/<parent>/<file>" — the
            // leading slash anchors the match to a component boundary.
            let suffix = normalized
                .rmatch_indices('/')
                .nth(1)
                .map(|(index, _)| &normalized[index..])
                .unwrap_or(&normalized);

            haystack.contains(&format!("{suffix}(")) || haystack.contains(&format!("{suffix} :"))
        })
        .cloned()
        .collect()
}

/// Apply the resolved SDK environment to an MSBuild invocation.
#[cfg(feature = "wasm")]
fn with_eval_env(
    mut input: moon_pdk_api::ExecCommandInput,
    env: &EvalEnv,
) -> moon_pdk_api::ExecCommandInput {
    if let Some(root) = &env.dotnet_root {
        input.env.insert("DOTNET_ROOT".into(), root.clone());
        input
            .paths
            .push(moon_pdk_api::VirtualPath::Real(root.into()));
    }

    // Only an explicit executable path redirects which SDK evaluates; see
    // `EvalEnv::dotnet_exe`.
    if let Some(exe) = &env.dotnet_exe {
        input.command = exe.clone();
    }

    if let Some(cwd) = &env.cwd {
        input.cwd = Some(cwd.clone());
    }

    input
}

/// Evaluate many projects with a single MSBuild invocation, paying the
/// dotnet/MSBuild startup cost (which dominates per-project evaluation)
/// once instead of once per project, and evaluating in parallel. The
/// generated traversal files live under `.moon/cache/` in the workspace.
#[cfg(feature = "wasm")]
pub fn evaluate_projects_batch(
    workspace_root: &moon_pdk_api::VirtualPath,
    project_real_paths: &[std::path::PathBuf],
    eval_env: &EvalEnv,
) -> AnyResult<BTreeMap<String, MsbuildEvaluation>> {
    use moon_pdk::exec;
    use moon_pdk_api::{ExecCommandInput, anyhow};
    use starbase_utils::fs;

    // Known constraint: both scratch files use fixed names here, so two moon
    // processes building a graph in the same checkout at the same time can have
    // one truncating `traversal.proj` while the other's MSBuild reads it. A
    // per-invocation subdirectory would fix it, but wasm has no pid, clock or
    // randomness to name one with, and `MoonContext` offers only `working_dir`
    // and `workspace_root` — a name derived from the project set would still
    // collide for the identical batch. The failure is a malformed traversal
    // project, which surfaces as a batch failure and falls back to per-project
    // evaluation, so it degrades rather than corrupting results.
    let dir = workspace_root
        .join(".moon")
        .join("cache")
        .join("dotnet-toolchain");

    fs::create_dir_all(&dir)?;
    fs::write_file(dir.join("moon-eval.targets"), moon_eval_targets_xml())?;

    let traversal = dir.join("traversal.proj");

    let traversal_arg = traversal
        .real_path()
        .ok_or_else(|| anyhow!("no host-real path for {traversal:?}"))?
        .to_string_lossy()
        .to_string();

    let run = |batch_paths: &[String]| {
        fs::write_file(&traversal, traversal_project_xml(batch_paths))?;

        exec(with_eval_env(
            ExecCommandInput::pipe(
                "dotnet",
                [
                    "msbuild",
                    traversal_arg.as_str(),
                    "-nologo",
                    // Parallel in-process worker nodes, but never leave them
                    // alive after the invocation (node reuse lingers ~15 min,
                    // which is hostile to CI containers).
                    "-maxCpuCount",
                    "-nodeReuse:false",
                    "-t:MoonCollect",
                    "-getItem:MoonEval",
                ],
            ),
            eval_env,
        ))
    };

    let paths = project_real_paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    let output = run(&paths)?;

    if output.exit_code == 0 {
        return parse_batch_output(&output.stdout);
    }

    // MSBuild returns NO target outputs at all when any project fails to
    // load (ContinueOnError does not rescue load errors). Identify the
    // offenders from the error lines and retry once without them — their
    // absence from the result triggers the caller's per-project fallback,
    // which surfaces the real error.
    let combined = format!("{}{}", output.stdout, output.stderr);
    let failed = detect_failed_projects(&combined, &paths);

    if !failed.is_empty() && failed.len() < paths.len() {
        let remaining = paths
            .iter()
            .filter(|path| !failed.contains(path))
            .cloned()
            .collect::<Vec<_>>();

        let retry = run(&remaining)?;

        if retry.exit_code == 0 {
            return parse_batch_output(&retry.stdout);
        }
    }

    Err(anyhow!(
        "Batched MSBuild evaluation failed (exit code {}): {}{}",
        output.exit_code,
        output.stdout,
        output.stderr,
    ))
}

/// Run a real MSBuild evaluation for a project file (host-real path).
#[cfg(feature = "wasm")]
pub fn evaluate_project(
    csproj_real_path: &std::path::Path,
    eval_env: &EvalEnv,
) -> AnyResult<MsbuildEvaluation> {
    use moon_pdk::exec;
    use moon_pdk_api::{ExecCommandInput, anyhow};

    let path_arg = csproj_real_path.to_string_lossy().to_string();

    let output = exec(with_eval_env(
        ExecCommandInput::pipe(
            "dotnet",
            [
                "msbuild",
                path_arg.as_str(),
                "-nologo",
                &format!("-getProperty:{EVAL_PROPERTIES}"),
                &format!("-getItem:{EVAL_ITEMS}"),
            ],
        ),
        eval_env,
    ))?;

    if output.exit_code != 0 {
        return Err(anyhow!(
            "MSBuild evaluation failed for {} (exit code {}): {}{}",
            csproj_real_path.display(),
            output.exit_code,
            output.stdout,
            output.stderr,
        ));
    }

    parse_msbuild_output(&output.stdout)
}
