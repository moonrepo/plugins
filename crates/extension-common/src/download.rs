use extism_pdk::debug;
use moon_pdk::{AnyResult, VirtualPath, anyhow, fetch_bytes};
use starbase_utils::fs;

pub fn download_from_url<U: AsRef<str>, P: AsRef<VirtualPath>>(
    src_url: U,
    dst_dir: P,
    file_name: Option<&str>,
) -> AnyResult<VirtualPath> {
    let url = src_url.as_ref();
    let dir = dst_dir.as_ref();

    debug!("Downloading file from <url>{}</url>", url);

    // Extract the file name from the URL
    let file_name = match file_name {
        Some(name) => name,
        None => match url.rfind('/') {
            Some(index) => &url[index + 1..],
            None => {
                return Err(anyhow!(
                    "Unable to determine a file name from <url>{url}</url>."
                ));
            }
        },
    };

    // Fetch the bytes of the URL
    let bytes = fetch_bytes(url)?;

    // Write the to the provided file
    let file = dir.join(file_name);

    fs::create_dir_all(dir)?;
    fs::write_file(&file, bytes)?;

    debug!("Downloaded to <path>{file}</path>");

    Ok(file)
}
