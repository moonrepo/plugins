use crate::config::DotnetToolchainConfig;
use crate::discovery::{SKIP_DIRS, installed_sdk_versions};
use crate::dotnet_install::{
    exact_version, install_script_file_name, install_script_url, install_version_args,
};
use crate::global_json::{parse_sdk_requirement, satisfies};
use extism_pdk::*;
use moon_pdk::{
    HostLogInput, HostLogTarget, exec, fetch_text, get_host_environment, host_log,
    into_virtual_path, parse_toolchain_config, plugin_err,
};
use moon_pdk_api::*;
use starbase_utils::fs;

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

/// Collect `global.json` files in the workspace (depth-limited). Unlike task
/// environments, setup has no project to walk up from — and the pin often
/// lives in a subtree (`src/backend/global.json`) rather than at the root.
fn collect_global_json_files(dir: &VirtualPath, depth: u8, out: &mut Vec<VirtualPath>) {
    let Ok(entries) = fs::read_dir(dir.any_path()) else {
        return;
    };

    let mut subdirs = vec![];

    for entry in entries {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };

        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if depth > 0
                && !SKIP_DIRS
                    .iter()
                    .any(|skip| skip.eq_ignore_ascii_case(&name))
            {
                subdirs.push(name);
            }
        } else if name.eq_ignore_ascii_case("global.json") {
            out.push(dir.join(&name));
        }
    }

    for name in subdirs {
        collect_global_json_files(&dir.join(name), depth - 1, out);
    }
}

/// Warn when the SDKs now present in the install root cannot serve a
/// `global.json` pin in the workspace. Installing 8.0 while a subtree pins
/// 10.x is a silent misconfiguration otherwise: setup succeeds and every task
/// in that subtree fails later with the host's own error.
///
/// A warning rather than an error: the pinned subtree may deliberately rely
/// on a system-wide SDK instead of the one moon manages.
fn warn_on_unsatisfied_pins(
    workspace_root: &VirtualPath,
    install_root: &VirtualPath,
) -> AnyResult<()> {
    let mut files = vec![];
    collect_global_json_files(workspace_root, 4, &mut files);

    if files.is_empty() {
        return Ok(());
    }

    let installed = installed_sdk_versions(install_root);

    for file in files {
        let Ok(content) = fs::read_file(&file) else {
            continue;
        };

        let Some(requirement) = parse_sdk_requirement(&content) else {
            continue;
        };

        if !satisfies(&installed, &requirement) {
            host_log!(
                warn,
                "<path>{}</path> pins .NET SDK <symbol>{}</symbol>, which the installed SDKs do not satisfy ({}). Tasks under that directory will fail until the pinned SDK is installed — set <property>version</property> to a matching value.",
                file,
                requirement.version,
                if installed.is_empty() {
                    "none installed".to_owned()
                } else {
                    installed.join(", ")
                }
            );
        }
    }

    Ok(())
}

#[plugin_fn]
pub fn setup_toolchain(
    Json(input): Json<SetupToolchainInput>,
) -> FnResult<Json<SetupToolchainOutput>> {
    let mut output = SetupToolchainOutput::default();

    // Without a `version:` setting moon skips the setup action entirely
    // ("use globals on PATH"); stay a no-op if called anyway.
    let Some(spec) = &input.configured_version else {
        return Ok(Json(output));
    };

    let config = parse_toolchain_config::<DotnetToolchainConfig>(input.toolchain_config)?;
    let env = get_host_environment()?;
    let windows = env.os.is_windows();

    // Install root: explicit `dotnetRoot` config wins, else `~/.dotnet` —
    // the same order resolve_dotnet_root uses when injecting DOTNET_ROOT
    // into task environments, so installed SDKs are picked up without any
    // further configuration. SDK versions install side-by-side.
    let install_root: std::path::PathBuf = match &config.dotnet_root {
        Some(root) => root.into(),
        None => {
            let Some(home) = env.home_dir.real_path() else {
                return Err(plugin_err!(
                    "Unable to resolve the host home directory for the default `~/.dotnet` install root."
                ));
            };

            home.join(".dotnet")
        }
    };

    let version_args = match install_version_args(spec, windows) {
        Ok(args) => args,
        Err(message) => return Err(plugin_err!("{}", message)),
    };

    // Fully-qualified versions can skip the network entirely when that SDK
    // is already laid out. Channels/aliases resolve server-side, so the
    // install script decides for those (it skips re-installs itself).
    if let Some(version) = exact_version(spec)
        && into_virtual_path(install_root.join("sdk").join(&version))?.exists()
    {
        warn_on_unsatisfied_pins(
            &input.context.workspace_root,
            &into_virtual_path(&install_root)?,
        )?;

        return Ok(Json(output));
    }

    // Stage the official install script under moon's cache dir, fetched once.
    // moon does not fingerprint-cache this action — `setup_toolchain` uses
    // `create_hash_and_return_lock`, which unlike the `_if_changed` variant has
    // no "manifest exists, skip" short-circuit — so this function runs on every
    // moon invocation. Re-downloading each time made every command depend on
    // reaching dot.net, which breaks offline and air-gapped workspaces
    // outright. Delete the file to force a re-fetch.
    let script_file = input
        .context
        .workspace_root
        .join(".moon/cache/dotnet-toolchain")
        .join(install_script_file_name(windows));

    if !script_file.exists() {
        if let Some(parent) = script_file.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write_file(&script_file, fetch_text(install_script_url(windows))?)?;
    }

    let Some(script_path) = script_file.real_path() else {
        return Err(plugin_err!(
            "Unable to resolve a host path for the staged install script."
        ));
    };

    // `--no-path`: task environments get DOTNET_ROOT/PATH injected by
    // extend_task_command; the user's shell profile is left alone.
    let mut args: Vec<String> = if windows {
        vec![
            "-NoProfile".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            script_path.to_string_lossy().to_string(),
            "-InstallDir".into(),
            install_root.to_string_lossy().to_string(),
            "-NoPath".into(),
        ]
    } else {
        vec![
            script_path.to_string_lossy().to_string(),
            "--install-dir".into(),
            install_root.to_string_lossy().to_string(),
            "--no-path".into(),
        ]
    };

    args.extend(version_args);

    let command = if windows { "powershell.exe" } else { "bash" };

    // Known limitation: for a channel or alias (`version: '8.0'`, `'lts'`) the
    // script still runs on every invocation, because only the server can say
    // which patch a channel currently resolves to. It exits early once that SDK
    // is present, but it does need the network to find out. Pin a
    // fully-qualified version to take the exact-version path above and skip
    // this entirely.
    let mut operation = Operation::new("install-sdk")?;
    let result = exec(ExecCommandInput::pipe(command, args))?;

    if result.exit_code != 0 {
        operation.finish(OperationStatus::Failed);
        output.operations.push(operation);

        return Err(plugin_err!(
            "dotnet-install failed with exit code {}:\n{}\n{}",
            result.exit_code,
            result.stdout,
            result.stderr,
        ));
    }

    operation.finish(OperationStatus::Passed);
    output.operations.push(operation);

    warn_on_unsatisfied_pins(
        &input.context.workspace_root,
        &into_virtual_path(&install_root)?,
    )?;

    // Informational only: for WASM-only toolchains the host currently
    // derives the action status itself and merges just operations/files.
    output.installed = true;

    Ok(Json(output))
}
