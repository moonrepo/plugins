// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::*;
use extism_pdk::*;
use moon_pdk::{VirtualPathExt, parse_toolchain_config};
use moon_pdk_api::*;

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
            && let Some(prof_dir) = project_root.join(".moon").to_real_path()?
        {
            match profile {
                NodeProfileType::Cpu => {
                    args.extend(vec![
                        "--cpu-prof".into(),
                        "--cpu-prof-name".into(),
                        "snapshot.cpuprofile".into(),
                        "--cpu-prof-dir".into(),
                        prof_dir.to_string(),
                    ]);
                }
                NodeProfileType::Heap => {
                    args.extend(vec![
                        "--heap-prof".into(),
                        "--heap-prof-name".into(),
                        "snapshot.heapprofile".into(),
                        "--heap-prof-dir".into(),
                        prof_dir.to_string(),
                    ]);
                }
            };
        }

        if !args.is_empty() {
            output.args = Some(Extend::Prepend(args));
        }
    }

    Ok(Json(output))
}
