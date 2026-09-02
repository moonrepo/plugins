//! Resolving which SDK to use, and the environment tasks run under.
//!
//! Task environments and MSBuild evaluation must agree on a `DOTNET_ROOT`, or
//! the project graph gets evaluated by one SDK while tasks run under another.
//! Everything that decides that lives here, alongside the two tier-2 functions
//! that materialize an environment.

use crate::config::DotnetToolchainConfig;
use crate::discovery::{installed_sdk_versions, proto_sdk_roots, walk_up};
use crate::global_json::{SdkRequirement, parse_sdk_requirement, satisfies, selects_test_platform};
use crate::msbuild::EvalEnv;
use extism_pdk::*;
use moon_pdk::{
    HostLogInput, HostLogTarget, VirtualPathExt, command_exists, get_host_env_var,
    get_host_environment, host_log,
};
use moon_pdk_api::*;
use starbase_utils::{fs, yaml};

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

/// Has moon been told to install a .NET SDK itself, via `version:` under
/// `dotnet` in `.moon/toolchains.yml`?
///
/// That is a moon-level toolchain setting, not one of ours, so it never reaches
/// the plugin through `toolchain_config` — `setup_toolchain` receives it as
/// `configured_version`, but the project graph is built before any of that runs.
/// Reading the file is the only way to know at graph-build time, and it decides
/// whether an unresolvable SDK is a terminal misconfiguration or simply an SDK
/// that has not been installed yet.
pub fn sdk_install_configured(workspace_root: &VirtualPath) -> bool {
    let file = workspace_root.join(".moon").join("toolchains.yml");

    if !file.exists() {
        return false;
    }

    // Untyped: `version` may be a string (`'8.0'`), a bare YAML float (`8.0`) or
    // an alias (`lts`), and only its presence matters here.
    yaml::read_file::<serde_json::Value>(file)
        .ok()
        .and_then(|root| {
            root.get("dotnet")
                .and_then(|section| section.get("version"))
                .cloned()
        })
        .is_some_and(|version| !version.is_null())
}

/// The one `global.json` governing a directory, with its contents: searching
/// from `start` up to (and including) the workspace root, the same direction the
/// dotnet host searches from its working directory.
///
/// The search stops at the first `global.json` that exists, whether or not it
/// declares the field the caller wants, because that is the one file the dotnet
/// host resolves — it neither merges them nor keeps looking. Walking past a file
/// that lacks an `sdk.version` would attribute an ancestor's pin to a directory
/// it does not govern, and name that non-governing file in the diagnostics.
///
/// Every caller goes through here so that rule holds in one place: the SDK pin
/// and the test-runner selection must agree on *which* file governs a directory,
/// or a project can be pinned by one file and have its `dotnet test` flavour
/// decided by another.
fn nearest_global_json(
    start: &VirtualPath,
    workspace_root: &VirtualPath,
) -> Option<(VirtualPath, String)> {
    for dir in walk_up(start, workspace_root) {
        let file = dir.join("global.json");

        if file.exists() {
            // An unreadable file is treated as governing but saying nothing,
            // which is what stopping the walk already implies.
            return Some((file.clone(), fs::read_file(&file).unwrap_or_default()));
        }
    }

    None
}

/// Nearest `global.json` SDK pin. Returns the file path (for messages) and the
/// parsed pin. See [`nearest_global_json`] for which file that is.
pub fn find_sdk_requirement(
    start: &VirtualPath,
    workspace_root: &VirtualPath,
) -> Option<(String, SdkRequirement)> {
    let (file, content) = nearest_global_json(start, workspace_root)?;

    parse_sdk_requirement(&content).map(|requirement| (file.to_string(), requirement))
}

/// Does the `global.json` governing this directory select
/// Microsoft.Testing.Platform for `dotnet test`?
pub fn uses_test_platform_runner(start: &VirtualPath, workspace_root: &VirtualPath) -> bool {
    nearest_global_json(start, workspace_root)
        .is_some_and(|(_, content)| selects_test_platform(&content))
}

/// Where to look for a `global.json` SDK pin when validating a discovered SDK
/// root: from `start` up to (and including) `workspace_root`.
#[derive(Clone, Copy)]
pub struct SdkPinScope<'a> {
    pub start: &'a VirtualPath,
    pub workspace_root: &'a VirtualPath,
}

/// Resolve the DOTNET_ROOT for task environments *and* MSBuild evaluation —
/// both must agree, or the graph gets evaluated by one SDK while tasks run
/// under another.
///
/// Order: existing host env var > proto's own installs > `~/.dotnet` when it
/// holds a real SDK layout.
///
/// Tasks would manage without the proto step, since moon puts the resolved tool
/// directory on their PATH. Graph building would not: it shells out to `dotnet`
/// from the host PATH and is handed no tool directory, so an SDK proto installed
/// is otherwise invisible to inference.
///
/// Both discovered candidates are guarded against the workspace's `global.json`
/// pin. An install that cannot serve the pin would make every task fail with the
/// host's own error, so it is skipped in favour of whatever `dotnet` is on PATH.
/// An explicit `DOTNET_ROOT` is never second-guessed.
/// Deliberately not memoized in a plugin var, even though it runs once per task
/// and once per manifest. The answer is not stable for the lifetime of the
/// process: the project graph is built *before* `SetupToolchain`, so graph
/// building legitimately resolves one root (an existing `~/.dotnet`, or none at
/// all) and proto then installs the SDK that should outrank it. A cached miss
/// would leave every later task without a `DOTNET_ROOT` for the SDK moon had
/// just installed, and a cached hit would pin them to `~/.dotnet` while the
/// muxer on their PATH is proto's. Either way evaluation and execution end up
/// under different SDKs, which is the one thing this function exists to prevent.
fn resolve_dotnet_root(scope: SdkPinScope<'_>) -> AnyResult<Option<String>> {
    if let Some(existing) = get_host_env_var("DOTNET_ROOT")?
        && !existing.is_empty()
    {
        return Ok(Some(existing));
    }

    let env = get_host_environment()?;

    // proto's installs come first: this plugin asked for them, so they are the
    // SDK it should be evaluating and running against. `~/.dotnet` last, and it
    // doubles as the dotnet CLI's user-level cache directory, so mere existence
    // is not enough — `usable_root` requires the host executable.
    let mut candidates = proto_sdk_roots()
        .into_iter()
        .map(|root| (root, "proto"))
        .collect::<Vec<_>>();

    candidates.push((env.home_dir.join(".dotnet"), "~/.dotnet"));

    // Rejecting a candidate over the pin is only an improvement when something
    // else can take its place. That is either a later candidate or a `dotnet` on
    // PATH — checking only the latter meant a proto-managed machine with no
    // system SDK skipped the pin check entirely, and so took the newest install
    // even when an older one beside it was the one the pin asked for.
    let path_fallback = command_exists(env, "dotnet");
    let last = candidates.len().saturating_sub(1);

    for (index, (root, label)) in candidates.into_iter().enumerate() {
        let enforce_pin = index < last || path_fallback;

        if let Some(root) = usable_root(&root, env, scope, label, enforce_pin)? {
            return Ok(Some(root));
        }
    }

    Ok(None)
}

/// Validate a discovered SDK root and convert it to a host path.
///
/// A root counts only when it holds the `dotnet` host executable, and — when
/// `enforce_pin` says another root can take its place — an SDK satisfying the
/// workspace pin.
fn usable_root(
    root: &VirtualPath,
    env: &HostEnvironment,
    scope: SdkPinScope<'_>,
    label: &str,
    enforce_pin: bool,
) -> AnyResult<Option<String>> {
    if !root.join(env.os.get_exe_name("dotnet")).exists() {
        return Ok(None);
    }

    if enforce_pin
        && let Some((file, requirement)) = find_sdk_requirement(scope.start, scope.workspace_root)
    {
        let installed = installed_sdk_versions(root);

        if !satisfies(&installed, &requirement) {
            host_log!(
                warn,
                "Ignoring the <path>{}</path> SDK for DOTNET_ROOT: it has no SDK satisfying <symbol>{}</symbol> from <path>{}</path> (found: {}). Looking for another .NET installation instead.",
                label,
                requirement.version,
                file,
                if installed.is_empty() {
                    "none".to_owned()
                } else {
                    installed.join(", ")
                }
            );

            return Ok(None);
        }
    }

    if let Some(real) = root.to_real_path()? {
        let path = real.to_string_lossy().to_string();

        host_log!(
            debug,
            "Using the <path>{}</path> SDK as DOTNET_ROOT: <path>{}</path>",
            label,
            path
        );

        return Ok(Some(path));
    }

    Ok(None)
}

/// Build the MSBuild evaluation environment: the same DOTNET_ROOT tasks get,
/// plus an explicit working directory (`global.json` resolves from there).
pub fn build_eval_env(
    config: &DotnetToolchainConfig,
    cwd: VirtualPath,
    workspace_root: &VirtualPath,
) -> AnyResult<EvalEnv> {
    let dotnet_root = resolve_dotnet_root(SdkPinScope {
        start: &cwd,
        workspace_root,
    })?;

    // Point at the muxer inside the root, but only when its existence can be
    // confirmed — a host path must be virtualized before wasm can stat it,
    // and roots outside the plugin's readable paths cannot be checked at all.
    // Guessing would turn a working evaluation into "command not found", so
    // unverifiable roots keep using the `dotnet` on PATH, as before.
    let dotnet_exe = dotnet_root.as_ref().and_then(|root| {
        let env = get_host_environment().ok()?;
        let real = std::path::PathBuf::from(root).join(env.os.get_exe_name("dotnet"));

        VirtualPath::create(&real)
            .ok()?
            .exists()
            // The host converts a command containing a separator back from
            // its virtual form, so pass the real path.
            .then(|| real.to_string_lossy().to_string())
    });

    Ok(EvalEnv {
        dotnet_root,
        dotnet_exe,
        cwd: Some(cwd),
        msbuild_properties: config.msbuild_properties.clone(),
    })
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();

    // Tasks run in their project directory, so that is where the dotnet host
    // resolves `global.json` from — validate the fallback against that pin.
    let project_root = input.context.get_project_root(&input.project);
    let scope = SdkPinScope {
        start: &project_root,
        workspace_root: &input.context.workspace_root,
    };

    // Deliberately only DOTNET_ROOT and PATH. Injecting vendor environment
    // variables nobody asked for is surprising, and the
    // `DOTNET_CLI_TELEMETRY_OPTOUT` that used to be set here was set *inside*
    // this branch — so it never applied in the common case of a system SDK on
    // PATH with no DOTNET_ROOT. It also does not suppress the "Welcome to .NET"
    // first-run banner, which is what `DOTNET_NOLOGO` controls. Both belong in a
    // task's own `env`, where they are visible.
    if let Some(root) = resolve_dotnet_root(scope)? {
        output.env.insert("DOTNET_ROOT".into(), root.clone());
        output.paths.push(root.into());
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let mut output = SetupEnvironmentOutput::default();

    // Restore local dotnet tools once per dependencies root when a tool
    // manifest exists. Local tools (.config/dotnet-tools.json) are distinct
    // from global tools, which remain out of scope.
    //
    // Search from the dependencies root up to the workspace root, the same
    // way the dotnet CLI resolves a tool manifest: it conventionally lives at
    // the repository root, which is not necessarily a dependencies root (any
    // project directory holding a lock file becomes one).
    let mut tool_manifest = None;

    for dir in walk_up(&input.root, &input.context.workspace_root) {
        let candidate = dir.join(".config").join("dotnet-tools.json");

        if candidate.exists() {
            tool_manifest = Some(candidate);
            break;
        }
    }

    if let Some(tool_manifest) = tool_manifest {
        let mut command = ExecCommand::new(
            ExecCommandInput::new("dotnet", ["tool", "restore"]).cwd(input.root.clone()),
        );

        // The label has to stay stable: it *is* the on-disk cache key
        // (`<prefix>:<label>`). What re-runs the restore is the fingerprint
        // stored under that key, and `inputs` is what puts the manifest's
        // content hash into it — so a manifest edit re-restores, while an
        // unrelated re-run of the action does not.
        command.label = Some("dotnet tool restore".into());
        command.cache = Some(CacheStrategy::Hash);
        command.inputs.push(CacheInput::FileHash(tool_manifest));

        output.commands.push(command);
    }

    Ok(Json(output))
}
