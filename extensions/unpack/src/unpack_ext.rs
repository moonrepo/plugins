use extension_common::download::download_from_url;
use extension_common::format_virtual_path;
use extism_pdk::*;
use moon_pdk::*;
use starbase_archive::Archiver;
use std::fs;

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
        name: "Unpack".into(),
        description: Some("Unpack an archive into the provided destination.".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_schema: None,
    }))
}

#[derive(Args)]
pub struct UnpackExtensionArgs {
    #[arg(long, short = 's', required = true)]
    pub src: String,

    #[arg(long, short = 'd')]
    pub dest: Option<String>,

    #[arg(long)]
    pub prefix: Option<String>,
}

#[plugin_fn]
pub fn execute_extension(Json(input): Json<ExecuteExtensionInput>) -> FnResult<()> {
    let args = parse_args::<UnpackExtensionArgs>(&input.args)?;

    // Determine the correct input. If the input is a URL, attempt to download
    // the file, otherwise use the file directly (if within our whitelist).
    let src_file = if args.src.starts_with("http") {
        debug!("Received a URL as the input source");

        download_from_url(&args.src, virtual_path!("/moon/temp"), None)?
    } else {
        debug!(
            "Converting source <file>{}</file> to an absolute virtual path",
            args.src
        );

        virtual_path!(buf, input.context.get_absolute_path(args.src))
    };

    if !src_file
        .extension()
        .is_some_and(|ext| ext == "tar" || ext == "tgz" || ext == "gz" || ext == "zip")
    {
        return Err(plugin_err!(
            "Invalid source, only <file>.tar</file>, <file>.tar.gz</file>, and <file>.zip</file> archives are supported."
        ));
    }

    if !src_file.exists() || !src_file.is_file() {
        return Err(plugin_err!(
            "Source <path>{}</path> must be a valid file.",
            format_virtual_path(&src_file),
        ));
    }

    host_log!(
        stdout,
        "Opening archive <path>{}</path>",
        format_virtual_path(&src_file),
    );

    // Convert the provided output into a virtual file path.
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

    fs::create_dir_all(&dest_dir)?;

    host_log!(
        stdout,
        "Unpacking archive to <path>{}</path>",
        format_virtual_path(&dest_dir),
    );

    // Attempt to unpack the archive!
    let mut archive = Archiver::new(&dest_dir, &src_file);

    // Diff against all files in the output dir
    archive.add_source_glob("**/*");

    // Remove the prefix from unpacked files
    if let Some(prefix) = &args.prefix {
        archive.set_prefix(prefix);
    }

    // Unpack the files
    if let Err(error) = archive.unpack_from_ext() {
        let mut message = error.to_string();

        // Miette hides the real error
        if let Some(source) = error.source() {
            message.push(' ');
            message.push_str(&source.to_string());
        }

        return Err(plugin_err!("{message}"));
    };

    host_log!(stdout, "Unpacked archive!");

    Ok(())
}
