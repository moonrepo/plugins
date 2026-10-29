use extism_pdk::debug;
use moon_pdk::{AnyResult, VirtualPath};
use starbase_utils::fs;

pub fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

pub fn download_from_url<U: AsRef<str>, P: AsRef<VirtualPath>>(
    src_url: U,
    dst_dir: P,
    file_name: Option<&str>,
) -> AnyResult<VirtualPath> {
    let url = src_url.as_ref();
    let dir = dst_dir.as_ref();

    debug!("Downloading file from <url>{}</url>", url);

    // Extract the file name from the URL, without the query or fragment
    let file_name = file_name.unwrap_or_else(|| {
        let path = url.split(['?', '#']).next().unwrap_or_default();

        path.rsplit('/').next().unwrap_or_default()
    });

    // Download on the host, so the file isn't loaded into memory
    let file = dir.join(file_name);

    fs::create_dir_all(dir)?;
    moon_pdk::download_from_url(url, &file)?;

    debug!("Downloaded to <path>{file}</path>");

    Ok(file)
}
