use crate::format_virtual_path;
use extism_pdk::debug;
use moon_pdk::{fetch_bytes, AnyResult, VirtualPath};
use std::fs;

pub fn download_from_url<U: AsRef<str>, P: AsRef<VirtualPath>>(
    src_url: U,
    dst_dir: P,
    file_name: Option<&str>,
) -> AnyResult<VirtualPath> {
    let url = src_url.as_ref();
    let dir = dst_dir.as_ref();

    debug!("Downloading file from <url>{}</url>", url);

    // Extract the file name from the URL
    let file_name = file_name.unwrap_or_else(|| &url[url.rfind('/').unwrap() + 1..]);

    // Fetch the bytes of the URL
    let bytes = fetch_bytes(url)?;

    // Write the to the provided file
    let file = dir.join(file_name);

    fs::create_dir_all(dir)?;
    fs::write(&file, bytes)?;

    debug!("Downloaded to <path>{}</path>", format_virtual_path(&file));

    Ok(file)
}
