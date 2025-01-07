use extension_common::download::download_from_url;
use extension_common::format_virtual_path;
use extism_pdk::*;
use moon_pdk::*;

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
    fn to_virtual_path(path: String) -> String;
}

#[plugin_fn]
pub fn register_extension(
    Json(_): Json<ExtensionMetadataInput>,
) -> FnResult<Json<ExtensionMetadataOutput>> {
    Ok(Json(ExtensionMetadataOutput {
        name: "Download".into(),
        description: Some("Download a file from a URL into the current working directory.".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_schema: None,
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

    let dest_dir = virtual_path!(
        buf,
        input
            .context
            .get_absolute_path(args.dest.as_deref().unwrap_or_default())
    );

    if dest_dir.exists() && dest_dir.is_file() {
        return Err(plugin_err!(
            "Destination <path>{}</path> must be a directory, found a file.",
            format_virtual_path(&dest_dir),
        ));
    }

    debug!(
        "Destination <path>{}</path> will be used",
        format_virtual_path(&dest_dir),
    );

    // Attempt to download the file
    host_log!(stdout, "Downloading <url>{}</url>", args.url);

    let dest_file = download_from_url(&args.url, &dest_dir, args.name.as_deref())?;

    host_log!(
        stdout,
        "Downloaded to <path>{}</path>",
        format_virtual_path(&dest_file),
    );

    Ok(())
}
