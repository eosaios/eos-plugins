use eos_plugin_protocol::*;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::Path,
};
use unicode_normalization::UnicodeNormalization;

pub const MAX_PACKAGE: u64 = 200 * 1024 * 1024;
pub const MAX_EXPANDED: u64 = 500 * 1024 * 1024;
pub const MAX_FILE: u64 = 200 * 1024 * 1024;
pub const MAX_FILES: usize = 10_000;
#[derive(Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub digest: String,
    pub length: u64,
    pub executable: bool,
}
pub struct Package {
    pub manifest: Manifest,
    pub manifest_digest: String,
    pub digest: String,
    pub files_digest: String,
    pub files: BTreeMap<String, Vec<u8>>,
    pub records: Vec<FileRecord>,
}
fn collision_key(path: &str) -> String {
    path.nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .replace('ß', "ss")
        .replace('ς', "σ")
}
pub fn read_package(bytes: &[u8]) -> Result<Package> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_PACKAGE {
        return Err(error(
            "QUOTA_EXCEEDED",
            "compressed package size out of bounds",
        ));
    }
    let decoder = GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder.take(MAX_EXPANDED + 32 * 1024 * 1024));
    let mut files = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    let mut total = 0u64;
    let entries = archive
        .entries()
        .map_err(|_| error("ARCHIVE_UNSAFE", "invalid tar.gz archive"))?;
    for (count, entry) in entries.enumerate() {
        if count >= MAX_FILES {
            return Err(error("QUOTA_EXCEEDED", "too many archive entries"));
        }
        let mut entry = entry.map_err(|_| error("ARCHIVE_UNSAFE", "damaged archive entry"))?;
        let directory = entry.header().entry_type().is_dir();
        if !directory && !entry.header().entry_type().is_file() {
            return Err(error(
                "ARCHIVE_UNSAFE",
                "only regular files and directories are allowed",
            ));
        }
        let raw = entry.path_bytes();
        let path = std::str::from_utf8(&raw)
            .map_err(|_| error("ARCHIVE_UNSAFE", "non UTF-8 archive path"))?
            .trim_end_matches('/')
            .to_owned();
        if !safe_path(&path) || !seen.insert(collision_key(&path)) {
            return Err(error(
                "ARCHIVE_UNSAFE",
                "unsafe, duplicate or colliding archive path",
            ));
        }
        let mode = entry
            .header()
            .mode()
            .map_err(|_| error("ARCHIVE_UNSAFE", "invalid file mode"))?;
        if mode & !0o777 != 0 {
            return Err(error("ARCHIVE_UNSAFE", "special permission bits forbidden"));
        }
        if directory {
            continue;
        }
        let length = entry.size();
        if length > MAX_FILE || length > MAX_EXPANDED.saturating_sub(total) {
            return Err(error("QUOTA_EXCEEDED", "expanded package exceeds limit"));
        }
        let mut content = Vec::with_capacity(length.min(1024 * 1024) as usize);
        (&mut entry)
            .take(length + 1)
            .read_to_end(&mut content)
            .map_err(|_| error("ARCHIVE_UNSAFE", "truncated archive"))?;
        if content.len() as u64 != length {
            return Err(error("ARCHIVE_UNSAFE", "file length mismatch"));
        }
        total += length;
        records.push(FileRecord {
            path: path.clone(),
            digest: digest(&content),
            length,
            executable: mode & 0o111 != 0,
        });
        files.insert(path, content);
    }
    for path in files.keys() {
        let parts: Vec<_> = path.split('/').collect();
        for n in 1..parts.len() {
            if files
                .keys()
                .any(|p| collision_key(p) == collision_key(&parts[..n].join("/")))
            {
                return Err(error("ARCHIVE_UNSAFE", "file/directory collision"));
            }
        }
    }
    let manifest_bytes = files
        .get("plugin.toml")
        .ok_or_else(|| error("SCHEMA_INVALID", "package root requires plugin.toml"))?;
    let manifest = parse_manifest(manifest_bytes)?;
    if !files.contains_key("LICENSE") || !files.contains_key("README.md") {
        return Err(error("SCHEMA_INVALID", "LICENSE and README.md required"));
    }
    for module in &manifest.modules {
        if let Some(entry) = &module.entry {
            let exists = if module.r#type == "skills/v1" {
                files
                    .keys()
                    .any(|p| p.starts_with(&format!("{entry}/")) && p.ends_with("/SKILL.md"))
            } else {
                files.contains_key(entry)
            };
            if !exists {
                return Err(error(
                    "SCHEMA_INVALID",
                    format!("missing module entry: {}", module.id),
                ));
            }
        }
    }
    for path in manifest
        .plugin
        .i18n
        .values()
        .chain(manifest.plugin.icon.iter())
        .chain(manifest.configuration.iter().map(|v| &v.schema))
        .chain(manifest.exports.iter().flat_map(|e| {
            e.methods
                .iter()
                .flat_map(|m| [&m.input_schema, &m.output_schema])
        }))
    {
        if !files.contains_key(path) {
            return Err(error(
                "SCHEMA_INVALID",
                format!("missing declared asset: {path}"),
            ));
        }
    }
    records.sort_by(|a, b| a.path.cmp(&b.path));
    for path in
        manifest
            .configuration
            .iter()
            .map(|c| &c.schema)
            .chain(manifest.exports.iter().flat_map(|e| {
                e.methods
                    .iter()
                    .flat_map(|m| [&m.input_schema, &m.output_schema])
            }))
    {
        let bytes = files
            .get(path)
            .ok_or_else(|| error("SCHEMA_INVALID", "declared schema is missing"))?;
        if bytes.len() > 1024 * 1024 {
            return Err(error("QUOTA_EXCEEDED", "JSON schema exceeds 1 MiB"));
        }
        let schema: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|_| error("SCHEMA_INVALID", "declared schema is not JSON"))?;
        eos_plugin_protocol::configuration_validator(&schema)?;
    }
    Ok(Package {
        manifest,
        manifest_digest: digest(manifest_bytes),
        digest: digest(bytes),
        files_digest: canonical_digest(&records)?,
        files,
        records,
    })
}
pub fn unpack_verified(package: &Package, destination: &Path) -> Result<()> {
    if destination.exists() {
        return Err(error(
            "RESOURCE_CONFLICT",
            "staging destination already exists",
        ));
    }
    fs::create_dir_all(destination).map_err(io_error)?;
    for record in &package.records {
        let path = destination.join(&record.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let bytes = package
            .files
            .get(&record.path)
            .ok_or_else(|| error("ARCHIVE_UNSAFE", "file inventory mismatch"))?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(io_error)?;
        file.write_all(bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                &path,
                fs::Permissions::from_mode(if record.executable { 0o700 } else { 0o600 }),
            )
            .map_err(io_error)?;
        }
    }
    Ok(())
}
pub fn pack_directory(root: &Path) -> Result<Vec<u8>> {
    let mut paths = Vec::new();
    fn walk(root: &Path, dir: &Path, paths: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(io_error)?;
            if metadata.file_type().is_symlink() {
                return Err(error("ARCHIVE_UNSAFE", "symlinks forbidden"));
            }
            if metadata.is_dir() {
                walk(root, &entry.path(), paths)?;
            } else if metadata.is_file() {
                let path = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| error("ARCHIVE_UNSAFE", "path escape"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                if !safe_path(&path) {
                    return Err(error("ARCHIVE_UNSAFE", "unsafe file name"));
                }
                paths.push(path);
            } else {
                return Err(error("ARCHIVE_UNSAFE", "special files forbidden"));
            }
        }
        Ok(())
    }
    walk(root, root, &mut paths)?;
    paths.sort();
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut archive = tar::Builder::new(encoder);
    for path in paths {
        let bytes = fs::read(root.join(&path)).map_err(io_error)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(root.join(&path))
                .map_err(io_error)?
                .permissions()
                .mode()
                & 0o111
                != 0
        };
        #[cfg(not(unix))]
        let executable = path.starts_with("native/");
        header.set_mode(if executable { 0o755 } else { 0o644 });
        header.set_cksum();
        archive
            .append_data(&mut header, &path, bytes.as_slice())
            .map_err(io_error)?;
    }
    let bytes = archive
        .into_inner()
        .map_err(io_error)?
        .finish()
        .map_err(io_error)?;
    read_package(&bytes)?;
    Ok(bytes)
}
