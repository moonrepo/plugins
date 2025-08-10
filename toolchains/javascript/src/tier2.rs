use extism_pdk::*;
use moon_pdk_api::*;
use std::path::PathBuf;

fn gather_shared_paths(
    context: &MoonContext,
    project: &ProjectFragment,
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    // Global packages
    if let Some(globals_dir) = globals_dir.and_then(|dir| dir.real_path()) {
        paths.push(globals_dir);
    }

    // Local packages upwards to the root
    let mut current_dir = context.get_project_root(project);

    while current_dir != context.workspace_root {
        if let Some(bin_dir) = current_dir.join("node_modules").join(".bin").real_path() {
            paths.push(bin_dir);
        }

        match current_dir.parent() {
            Some(dir) => {
                current_dir = dir;
            }
            None => {
                break;
            }
        }
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();

    gather_shared_paths(
        &input.context,
        &input.project,
        input.globals_dir.as_ref(),
        &mut output.paths,
    )?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    gather_shared_paths(
        &input.context,
        &input.project,
        input.globals_dir.as_ref(),
        &mut output.paths,
    )?;

    Ok(Json(output))
}
