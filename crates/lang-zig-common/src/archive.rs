use crate::ReleaseArtifact;
use proto_pdk_api::{AnyResult, Checksum, DownloadPrebuiltOutput, anyhow};

pub fn create_download_prebuilt_output(
    tool_name: &str,
    artifact: ReleaseArtifact,
) -> AnyResult<DownloadPrebuiltOutput> {
    let filename = artifact
        .tarball
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            anyhow!(
                "Invalid {tool_name} download URL <url>{}</url>.",
                artifact.tarball
            )
        })?;

    let archive_prefix = [".tar.xz", ".tar.gz", ".zip"]
        .into_iter()
        .find_map(|suffix| filename.strip_suffix(suffix))
        .ok_or_else(|| anyhow!("Unsupported {tool_name} archive <file>{filename}</file>."))?;

    Ok(DownloadPrebuiltOutput {
        archive_prefix: Some(archive_prefix.into()),
        checksum: Some(Checksum::sha256(artifact.shasum)),
        download_name: Some(filename.into()),
        download_url: artifact.tarball,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_download_output_from_tarballs() {
        let output = create_download_prebuilt_output(
            "Zig",
            ReleaseArtifact {
                shasum: "hash".into(),
                tarball: "https://example.com/zig-x86_64-linux-0.14.1.tar.xz".into(),
            },
        )
        .unwrap();

        assert_eq!(
            output.archive_prefix.as_deref(),
            Some("zig-x86_64-linux-0.14.1")
        );
        assert_eq!(
            output.download_name.as_deref(),
            Some("zig-x86_64-linux-0.14.1.tar.xz")
        );
        assert_eq!(
            output.download_url,
            "https://example.com/zig-x86_64-linux-0.14.1.tar.xz"
        );
    }

    #[test]
    fn supports_all_zig_archive_extensions() {
        for filename in ["zls.tar.xz", "zls.tar.gz", "zls.zip"] {
            let output = create_download_prebuilt_output(
                "ZLS",
                ReleaseArtifact {
                    shasum: "hash".into(),
                    tarball: format!("https://example.com/{filename}"),
                },
            )
            .unwrap();

            assert_eq!(output.archive_prefix.as_deref(), Some("zls"));
        }
    }
}
