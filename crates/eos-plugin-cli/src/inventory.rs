//! Read-only inventory for an explicit legacy root. Never executes or converts a package.
use eos_plugin_protocol::{error, io_error, Result};
use serde_json::{json, Value};
use std::path::Path;

pub fn inspect(root: &Path) -> Result<Value> {
    if !root.exists() {
        return Ok(json!({"exists":false,"entries":[],"changed":false}));
    }
    let root = std::fs::canonicalize(root).map_err(io_error)?;
    let mut entries = Vec::new();
    walk(&root, &root, 0, &mut entries)?;
    Ok(json!({"exists":true,"entries":entries,"changed":false}))
}
fn walk(root: &Path, dir: &Path, depth: usize, entries: &mut Vec<Value>) -> Result<()> {
    if depth > 16 || entries.len() > 10000 {
        return Err(error("QUOTA_EXCEEDED", "legacy inventory limit exceeded"));
    }
    for entry in std::fs::read_dir(dir).map_err(io_error)? {
        let path = entry.map_err(io_error)?.path();
        let meta = std::fs::symlink_metadata(&path).map_err(io_error)?;
        if meta.file_type().is_symlink() {
            return Err(error(
                "ARCHIVE_UNSAFE",
                "legacy inventory contains a symlink; review it explicitly",
            ));
        }
        if meta.is_dir() {
            walk(root, &path, depth + 1, entries)?;
            continue;
        }
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
        if !matches!(name, "plugin.toml" | "plugin.json") {
            continue;
        }
        if meta.len() > 1024 * 1024 {
            return Err(error("QUOTA_EXCEEDED", "legacy manifest exceeds 1 MiB"));
        }
        let bytes = std::fs::read(&path).map_err(io_error)?;
        entries.push(json!({"manifest":path.strip_prefix(root).map_err(|_|error("ARCHIVE_UNSAFE","legacy path escape"))?.to_string_lossy(),"digest":eos_plugin_protocol::digest(&bytes),"format":name,"disposition":"explicit-review-required"}));
    }
    Ok(())
}
