//! Resolving which SDK to use, and the environment tasks run under.
//!
//! Task environments and MSBuild evaluation must agree on a `DOTNET_ROOT`, or
//! the project graph gets evaluated by one SDK while tasks run under another.
//! Everything that decides that lives here, alongside the two tier-2 functions
//! that materialize an environment.

use crate::config::DotnetToolchainConfig;
use crate::discovery::{installed_sdk_versions, walk_up};
use crate::global_json::{SdkRequirement, parse_sdk_requirement, satisfies, selects_test_platform};
use crate::msbuild::EvalEnv;
use extism_pdk::*;
use moon_pdk::{
    HostLogInput, HostLogTarget, VirtualPathExt, command_exists, get_host_env_var,
    get_host_environment, host_log, parse_toolchain_config,
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

/// Nearest `global.json` SDK pin, searching from `start` up to (and
/// including) the workspace root — the same direction the dotnet host
/// searches from its working directory. Returns the file path (for messages)
/// and the parsed pin.
///
/// The search stops at the first `global.json` that exists, whether or not it
/// declares an `sdk.version`, because that is the one file the dotnet host
/// resolves — it neither merges them nor keeps looking. Walking past a pinless
/// file would attribute an ancestor's pin to a directory it does not govern, and
/// name that non-governing file in the diagnostics. `uses_test_platform_runner`
/// below already implements this rule; the two must agree.
pub fn find_sdk_requirement(
    start: &VirtualPath,
    workspace_root: &VirtualPath,
) -> Option<(String, SdkRequirement)> {
    for dir in walk_up(start, workspace_root) {
        let file = dir.join("global.json");

        if file.exists() {
            return fs::read_file(&file)
                .ok()
                .and_then(|content| parse_sdk_requirement(&content))
                .map(|requirement| (file.to_string(), requirement));
        }
    }

    None
}

/// Does the `global.json` governing this directory select
/// Microsoft.Testing.Platform for `dotnet test`? The nearest file wins,
/// whether or not it names a runner — the dotnet host resolves exactly one
/// `global.json`, it does not merge them.
pub fn uses_test_platform_runner(start: &VirtualPath, workspace_root: &VirtualPath) -> bool {
    for dir in walk_up(start, workspace_root) {
        let file = dir.join("global.json");

        if file.exists() {
            return fs::read_file(&file)
                .map(|content| selects_test_platform(&content))
                .unwrap_or(false);
        }
    }

    false
}

/// Where to look for a `global.json` SDK pin when validating the `~/.dotnet`
/// fallback: from `start` up to (and including) `workspace_root`.
pub struct SdkPinScope<'a> {
    pub start: &'a VirtualPath,
    pub workspace_root: &'a VirtualPath,
}

/// Resolve the DOTNET_ROOT for task environments *and* MSBuild evaluation —
/// both must agree, or the graph gets evaluated by one SDK while tasks run
/// under another.
///
/// Order: explicit config > existing host env var > `~/.dotnet` when it holds
/// a real SDK layout (where the proto dotnet plugin installs).
///
/// The `~/.dotnet` fallback is guarded: a leftover install there (a stale
/// proto experiment, say) would otherwise be injected over a perfectly good
/// system SDK, making every task fail against a `global.json` pin it cannot
/// satisfy. When a `dotnet` exists on PATH and the fallback cannot serve the
/// workspace's pin, the fallback is skipped so PATH wins. Explicit
/// configuration is never second-guessed.
fn resolve_dotnet_root(
    config: &DotnetToolchainConfig,
    scope: Option<SdkPinScope<'_>>,
) -> AnyResult<Option<String>> {
    if let Some(root) = &config.dotnet_root {
        return Ok(Some(root.clone()));
    }

    if let Some(existing) = get_host_env_var("DOTNET_ROOT")?
        && !existing.is_empty()
    {
        return Ok(Some(existing));
    }

    let env = get_host_environment()?;
    let candidate = env.home_dir.join(".dotnet");

    // `~/.dotnet` doubles as the dotnet CLI's user-level cache directory, so
    // mere existence is not enough — require the `dotnet` host executable,
    // which a real SDK install provides.
    let exe = if env.os.is_windows() {
        "dotnet.exe"
    } else {
        "dotnet"
    };

    if !candidate.join(exe).exists() {
        return Ok(None);
    }

    if let Some(scope) = scope
        && command_exists(env, "dotnet")
        && let Some((file, requirement)) = find_sdk_requirement(scope.start, scope.workspace_root)
    {
        let installed = installed_sdk_versions(&candidate);

        if !satisfies(&installed, &requirement) {
            host_log!(
                warn,
                "Ignoring the <path>~/.dotnet</path> fallback for DOTNET_ROOT: it has no SDK satisfying <symbol>{}</symbol> from <path>{}</path> (found: {}). Using the <symbol>dotnet</symbol> on PATH instead — set <property>dotnetRoot</property> to override.",
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

    if let Some(real) = candidate.to_real_path()? {
        let root = real.to_string_lossy().to_string();

        host_log!(
            debug,
            "Using the <path>~/.dotnet</path> fallback as DOTNET_ROOT: <path>{}</path>",
            root
        );

        return Ok(Some(root));
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
    let dotnet_root = resolve_dotnet_root(
        config,
        Some(SdkPinScope {
            start: &cwd,
            workspace_root,
        }),
    )?;

    // Point at the muxer inside the root, but only when its existence can be
    // confirmed — a host path must be virtualized before wasm can stat it,
    // and roots outside the plugin's readable paths cannot be checked at all.
    // Guessing would turn a working evaluation into "command not found", so
    // unverifiable roots keep using the `dotnet` on PATH, as before.
    let dotnet_exe = dotnet_root.as_ref().and_then(|root| {
        let env = get_host_environment().ok()?;
        let exe = if env.os.is_windows() {
            "dotnet.exe"
        } else {
            "dotnet"
        };
        let real = std::path::PathBuf::from(root).join(exe);

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
    let config = parse_toolchain_config::<DotnetToolchainConfig>(input.toolchain_config)?;
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
    if let Some(root) = resolve_dotnet_root(&config, Some(scope))? {
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
