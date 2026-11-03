//! Offline/worker-only TUF publisher. Online API callers never supply signing keys.
use aws_lc_rs::{rand::SystemRandom, signature::Ed25519KeyPair};
use eos_plugin_protocol::*;
use serde_json::json;
use std::{
    num::NonZeroU64,
    path::{Path, PathBuf},
};
use tough::{
    editor::{signed::SignedRole, RepositoryEditor},
    key_source::{KeySource, LocalKeySource},
    schema::{KeyHolder, Root},
};

pub async fn sign_repository(
    root: &Path,
    targets: &Path,
    metadata: &Path,
    revision: u64,
    key_paths: &[PathBuf],
) -> Result<()> {
    let version = NonZeroU64::new(revision)
        .ok_or_else(|| error("SIGNATURE_INVALID", "metadata revision must be positive"))?;
    let expiry = jiff::Timestamp::now()
        .checked_add(jiff::SignedDuration::from_secs(24 * 3600))
        .map_err(|_| error("SIGNATURE_INVALID", "invalid expiration"))?;
    let mut editor = RepositoryEditor::new(root)
        .await
        .map_err(|_| error("SIGNATURE_INVALID", "invalid trusted root"))?;
    editor
        .targets_version(version)
        .map_err(|_| error("SIGNATURE_INVALID", "targets version"))?;
    editor
        .targets_expires(expiry)
        .map_err(|_| error("SIGNATURE_INVALID", "targets expiration"))?;
    editor
        .snapshot_version(version)
        .snapshot_expires(expiry)
        .timestamp_version(version)
        .timestamp_expires(expiry);
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(targets).map_err(io_error)? {
        let p = entry.map_err(io_error)?.path();
        if !std::fs::symlink_metadata(&p).map_err(io_error)?.is_file() {
            return Err(error("ARCHIVE_UNSAFE", "target must be a regular file"));
        }
        paths.push(p);
    }
    paths.sort();
    editor
        .add_target_paths(paths)
        .await
        .map_err(|_| error("SIGNATURE_INVALID", "cannot inventory signed targets"))?;
    let keys: Vec<Box<dyn KeySource>> = key_paths
        .iter()
        .map(|path| Box::new(LocalKeySource { path: path.clone() }) as Box<dyn KeySource>)
        .collect();
    let signed = editor.sign(&keys).await.map_err(|_| {
        error(
            "SIGNATURE_INVALID",
            "signing keys do not satisfy root roles",
        )
    })?;
    signed
        .write(metadata)
        .await
        .map_err(|_| error("IO_FAILED", "cannot write TUF metadata"))?;
    let root_bytes = std::fs::read(root).map_err(io_error)?;
    std::fs::write(metadata.join("root.json"), &root_bytes).map_err(io_error)?;
    let root_value: serde_json::Value = serde_json::from_slice(&root_bytes)
        .map_err(|_| error("SIGNATURE_INVALID", "invalid root metadata"))?;
    let root_version = root_value["signed"]["version"]
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or_else(|| error("SIGNATURE_INVALID", "invalid root version"))?;
    if let Some(parent) = root.parent() {
        for entry in std::fs::read_dir(parent).map_err(io_error)? {
            let path = entry.map_err(io_error)?.path();
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if name
                .strip_suffix(".root.json")
                .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
            {
                let content = std::fs::read(&path).map_err(io_error)?;
                if content.len() > 1024 * 1024 {
                    return Err(error("QUOTA_EXCEEDED", "root metadata too large"));
                }
                std::fs::write(metadata.join(name), content).map_err(io_error)?;
            }
        }
    }
    std::fs::write(
        metadata.join(format!("{root_version}.root.json")),
        &root_bytes,
    )
    .map_err(io_error)?;
    Ok(())
}
/// Explicit isolated fixture ceremony. Never runs at server/client startup.
pub async fn init_development_repository(dir: &Path) -> Result<()> {
    if dir.exists() {
        return Err(error(
            "RESOURCE_CONFLICT",
            "development trust directory already exists",
        ));
    }
    std::fs::create_dir_all(dir).map_err(io_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(io_error)?;
    }
    let rng = SystemRandom::new();
    let mut keys = serde_json::Map::new();
    let mut roles = serde_json::Map::new();
    let mut root_ids = Vec::new();
    let mut root_sources: Vec<Box<dyn KeySource>> = Vec::new();
    use tough::sign::Sign;
    for name in [
        "root-1",
        "root-2",
        "root-3",
        "targets",
        "snapshot",
        "timestamp",
    ] {
        let document = Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|_| error("SIGNATURE_INVALID", "key generation failed"))?;
        let pair = Ed25519KeyPair::from_pkcs8(document.as_ref())
            .map_err(|_| error("SIGNATURE_INVALID", "generated key invalid"))?;
        let key = pair.tuf_key();
        let key_id = key
            .key_id()
            .map_err(|_| error("SIGNATURE_INVALID", "key ID failed"))?;
        let id = serde_json::to_value(&key_id)
            .map_err(|_| error("SIGNATURE_INVALID", "key ID encoding failed"))?
            .as_str()
            .ok_or_else(|| error("SIGNATURE_INVALID", "invalid key ID"))?
            .to_owned();
        keys.insert(
            id.clone(),
            serde_json::to_value(key)
                .map_err(|_| error("SCHEMA_INVALID", "key encoding failed"))?,
        );
        let path = dir.join(format!("{name}.pk8"));
        std::fs::write(&path, document.as_ref()).map_err(io_error)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .map_err(io_error)?;
        }
        if name.starts_with("root-") {
            root_ids.push(id);
            if name != "root-3" {
                root_sources.push(Box::new(LocalKeySource { path }));
            }
        } else {
            roles.insert(name.into(), json!({"threshold":1,"keyids":[id]}));
        }
    }
    roles.insert("root".into(), json!({"threshold":2,"keyids":root_ids}));
    let expires = jiff::Timestamp::now()
        .checked_add(jiff::SignedDuration::from_secs(365 * 86400))
        .map_err(|_| error("SIGNATURE_INVALID", "invalid expiration"))?
        .to_string();
    let root:Root=serde_json::from_value(json!({"_type":"root","spec_version":"1.0.0","consistent_snapshot":false,"version":1,"expires":expires,"keys":keys,"roles":roles})).map_err(|_|error("SIGNATURE_INVALID","invalid generated root"))?;
    let signed = SignedRole::new(root.clone(), &KeyHolder::Root(root), &root_sources, &rng)
        .await
        .map_err(|e| {
            error(
                "SIGNATURE_INVALID",
                format!("development root signing failed: {e}"),
            )
        })?;
    std::fs::write(dir.join("root.json"), signed.buffer()).map_err(io_error)?;
    Ok(())
}
