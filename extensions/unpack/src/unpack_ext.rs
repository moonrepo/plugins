use extension_common::download::download_from_url;
use extension_common::enable_tracing;
use extension_common::paths::get_absolute_path;
use extism_pdk::*;
use moon_pdk::*;
use moon_pdk_api::{ExecuteExtensionInput, RegisterExtensionInput, RegisterExtensionOutput};
use starbase_utils::fs;
use std::path::Path;

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

#[plugin_fn]
pub fn register_extension(
    Json(_): Json<RegisterExtensionInput>,
) -> FnResult<Json<RegisterExtensionOutput>> {
    enable_tracing();

    Ok(Json(RegisterExtensionOutput {
        name: "Unpack".into(),
        description: Some("Unpack an archive into the provided destination.".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
    }))
}

#[derive(Args)]
pub struct UnpackExtensionArgs {
    #[arg(long, short = 's', required = true)]
    pub src: String,

    #[arg(long, short = 'd')]
    pub dest: Option<String>,

    #[arg(long, default_value_t = 0)]
    pub components: usize,
}

#[plugin_fn]
pub fn execute_extension(Json(input): Json<ExecuteExtensionInput>) -> FnResult<()> {
    let args = parse_args::<UnpackExtensionArgs>(&input.args)?;

    // Determine the correct input. If the input is a URL, attempt to download
    // the file, otherwise use the file directly (if within our whitelist).
    let src_file = if args.src.starts_with("http") {
        debug!("Received a URL as the input source");

        download_from_url(&args.src, VirtualPath::new("/moon/temp"), None)?
    } else {
        debug!(
            "Converting source <file>{}</file> to an absolute virtual path",
            args.src
        );

        get_absolute_path(&input.context.working_dir, &args.src)
    };

    if !src_file.exists() || !src_file.is_file() {
        return Err(plugin_err!(
            "Source <path>{src_file}</path> must be a valid file.",
        ));
    }

    // Convert the provided output into a virtual file path.
    let dest_dir = get_absolute_path(
        &input.context.working_dir,
        args.dest.as_deref().unwrap_or_default(),
    );

    if dest_dir.exists() && dest_dir.is_file() {
        return Err(plugin_err!(
            "Destination <path>{dest_dir}</path> must be a directory, found a file.",
        ));
    }

    host_log!(stdout, "Unpacking archive to <path>{dest_dir}</path>",);

    fs::create_dir_all(&dest_dir)?;

    let is_zip = src_file
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));

    let env = get_host_environment()?;

    if is_zip && command_exists(env, "unzip") {
        unzip(&src_file, &dest_dir, args.components)?;
    } else {
        exec_unpack(
            "tar",
            [
                "-xf".into(),
                to_real_path(&src_file)?,
                "-C".into(),
                to_real_path(&dest_dir)?,
                format!("--strip-components={}", args.components),
            ],
        )?;
    }

    host_log!(stdout, "Unpacked archive!");

    Ok(())
}

fn unzip(src_file: &VirtualPath, dest_dir: &VirtualPath, components: usize) -> AnyResult<()> {
    if components == 0 {
        return exec_unpack(
            "unzip",
            [
                "-o".into(),
                to_real_path(src_file)?,
                "-d".into(),
                to_real_path(dest_dir)?,
            ],
        );
    }

    // Unzip can't strip components, so extract to a temporary directory,
    // and move the entries that are nested deep enough into the destination
    let temp_dir = dest_dir.join(".moon-unpack");

    fs::remove_dir_all(&temp_dir)?;

    let result = unzip(src_file, &temp_dir, 0).and_then(|_| {
        let mut dirs = vec![temp_dir.to_path_buf()];

        for _ in 0..components {
            let mut next_dirs = vec![];

            for dir in dirs {
                for entry in fs::read_dir(dir)? {
                    if entry.path().is_dir() {
                        next_dirs.push(entry.path());
                    }
                }
            }

            dirs = next_dirs;
        }

        for dir in dirs {
            move_dir_contents(&dir, dest_dir)?;
        }

        Ok(())
    });

    fs::remove_dir_all(&temp_dir)?;

    result
}

// Rename instead of copy, as WASI can't preserve file permissions
fn move_dir_contents(from: &Path, to: &Path) -> AnyResult<()> {
    for entry in fs::read_dir(from)? {
        let from_path = entry.path();
        let to_path = to.join(entry.file_name());

        if from_path.is_dir() && to_path.is_dir() {
            move_dir_contents(&from_path, &to_path)?;
        } else {
            fs::rename(from_path, to_path)?;
        }
    }

    Ok(())
}

fn exec_unpack(command: &str, args: impl IntoIterator<Item = String>) -> AnyResult<()> {
    let output = exec_streamed(command, args)?;

    if output.exit_code != 0 {
        return Err(anyhow!(
            "Failed to unpack archive, <shell>{command}</shell> exited with code {}.",
            output.exit_code
        ));
    }

    Ok(())
}

fn to_real_path(path: &VirtualPath) -> AnyResult<String> {
    path.to_real_path()?
        .map(|path| path.to_string())
        .ok_or_else(|| anyhow!("Unable to convert <path>{path}</path> to a real path."))
}
