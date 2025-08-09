// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::*;
use extism_pdk::*;
use moon_pdk::parse_toolchain_config;
use moon_pdk_api::*;
use starbase_utils::fs;
use std::path::PathBuf;

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let config = parse_toolchain_config::<NodeToolchainConfig>(input.toolchain_config)?;
    let mut output = SetupEnvironmentOutput::default();

    // Sync version manager
    if let Some(version_manager) = config.sync_version_manager_config
        && let Some(version) = config.version
    {
        let (op, file) = Operation::track("sync-version-manager", || {
            let rc_path = input.root.join(match version_manager {
                NodeVersionManager::Nodenv => ".node-version",
                NodeVersionManager::Nvm => ".nvmrc",
            });

            fs::write_file(&rc_path, version.to_string())?;

            Ok(rc_path)
        })?;

        output.operations.push(op);
        output.changed_files.extend(file.virtual_path());
    }

    Ok(Json(output))
}

fn gather_shared_paths(
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(globals_dir) = globals_dir {
        if let Some(value) = globals_dir.real_path() {
            paths.push(value);
        }
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let config = parse_toolchain_config::<NodeToolchainConfig>(input.toolchain_config)?;

    if input.command == "node" || input.command == "nodejs" {
        let mut args = config.execute_args;
        let project_root = input.context.get_project_root(&input.project);

        if let Some(profile) = &config.profile_execution
            && let Some(prof_dir) = project_root.join(".moon").real_path_string()
        {
            match profile {
                NodeProfileType::Cpu => {
                    args.extend(vec![
                        "--cpu-prof".into(),
                        "--cpu-prof-name".into(),
                        "snapshot.cpuprofile".into(),
                        "--cpu-prof-dir".into(),
                        prof_dir,
                    ]);
                }
                NodeProfileType::Heap => {
                    args.extend(vec![
                        "--heap-prof".into(),
                        "--heap-prof-name".into(),
                        "snapshot.heapprofile".into(),
                        "--heap-prof-dir".into(),
                        prof_dir,
                    ]);
                }
            };
        }

        if !args.is_empty() {
            output.args = Some(Extend::Prepend(args));
        }
    }

    gather_shared_paths(input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    gather_shared_paths(input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}
