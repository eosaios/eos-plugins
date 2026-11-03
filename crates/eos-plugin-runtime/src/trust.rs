//! TUF verification uses the audited tough implementation, never a package-supplied key.
use eos_plugin_protocol::*;
use futures_util::TryStreamExt;
use std::path::Path;
use tough::{RepositoryLoader, TargetName};
use url::Url;

pub async fn verify_tuf_target(
    root: &[u8],
    metadata_dir: &Path,
    targets_dir: &Path,
    datastore: &Path,
    target: &str,
) -> Result<Vec<u8>> {
    std::fs::create_dir_all(datastore).map_err(io_error)?;
    let metadata = Url::from_directory_path(metadata_dir)
        .map_err(|_| error("SIGNATURE_INVALID", "invalid metadata directory"))?;
    let targets = Url::from_directory_path(targets_dir)
        .map_err(|_| error("SIGNATURE_INVALID", "invalid targets directory"))?;
    let repository = RepositoryLoader::new(&root, metadata, targets)
        .datastore(datastore)
        .load()
        .await
        .map_err(|_| error("SIGNATURE_INVALID", "TUF metadata verification failed"))?;
    let name =
        TargetName::new(target).map_err(|_| error("SIGNATURE_INVALID", "invalid target name"))?;
    let mut stream = repository
        .read_target(&name)
        .await
        .map_err(|_| error("SIGNATURE_INVALID", "TUF target unavailable"))?
        .ok_or_else(|| error("SIGNATURE_INVALID", "target is not signed"))?;
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.try_next().await.map_err(|_| {
        error(
            "ARTIFACT_HASH_MISMATCH",
            "TUF target hash/length verification failed",
        )
    })? {
        if bytes.len() + chunk.len() > crate::MAX_PACKAGE as usize {
            return Err(error(
                "QUOTA_EXCEEDED",
                "signed target exceeds package limit",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
