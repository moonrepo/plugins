use extension_common::download::download_from_url;
use extension_common::enable_tracing;
use extism_pdk::*;
use moon_pdk::*;
use moon_pdk_api::{ExecuteExtensionInput, RegisterExtensionInput, RegisterExtensionOutput};

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
        name: "Download".into(),
        description: Some("Download a file from a URL into the current working directory.".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
    }))
}

#[derive(Args)]
pub struct DownloadExtensionArgs {
    #[arg(long, short = 'u', required = true)]
    pub url: String,

    #[arg(long, short = 'd')]
    pub dest: Option<String>,

    #[arg(long)]
    pub name: Option<String>,
}

#[plugin_fn]
pub fn execute_extension(Json(input): Json<ExecuteExtensionInput>) -> FnResult<()> {
    let args = parse_args::<DownloadExtensionArgs>(&input.args)?;

    if !args.url.starts_with("http") {
        return Err(plugin_err!("A valid URL is required for downloading."));
    }

    // Determine destination directory
    debug!("Determining destination directory");

    let dest_dir = input
        .context
        .get_absolute_path(args.dest.as_deref().unwrap_or_default());

    if dest_dir.exists() && dest_dir.is_file() {
        return Err(plugin_err!(
            "Destination <path>{dest_dir}</path> must be a directory, found a file.",
        ));
    }

    debug!("Destination <path>{dest_dir}</path> will be used",);

    // Attempt to download the file
    host_log!(stdout, "Downloading <url>{}</url>", args.url);

    let dest_file = download_from_url(&args.url, &dest_dir, args.name.as_deref())?;

    host_log!(stdout, "Downloaded to <path>{dest_file}</path>",);

    Ok(())
}
