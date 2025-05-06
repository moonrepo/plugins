use extism_pdk::*;
use moon_pdk::get_host_environment;
use moon_pdk_api::*;

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let command = &input.command;

    // Binary may be installed to `~/.cargo/bin` so we must prefix
    // it with `cargo` so that it can actually execute...
    if command != "cargo" &&
        command != "rls" &&
        // rustc, rustdoc, etc
        !command.starts_with("rust")
    {
        if let Some(globals_dir) = &input.globals_dir {
            let env = get_host_environment()?;
            let cargo_bin_name = command.strip_prefix("cargo-").unwrap_or(command);
            let cargo_bin_path =
                globals_dir.join(env.os.get_exe_name(format!("cargo-{cargo_bin_name}")));

            // Is a cargo executable, shift over arguments
            if cargo_bin_path.exists() {
                output.command = Some("cargo".into());
                output.args = Some(Extend::Prepend(vec![cargo_bin_name.into()]));
            }
        }
    }

    Ok(Json(output))
}
