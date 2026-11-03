//! Generated data contract plus strict parsing. Product rules do not belong here.
mod generated;
pub use generated::*;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fmt, sync::OnceLock};

pub type Result<T> = std::result::Result<T, ErrorBody>;
pub fn error(code: &str, message: impl Into<String>) -> ErrorBody {
    ErrorBody {
        code: code.into(),
        message: message.into(),
        details: Value::Null,
        retryable: false,
    }
}
impl fmt::Display for ErrorBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ErrorBody {}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn canonical_digest<T: Serialize>(value: &T) -> Result<String> {
    // serde_json maps are ordered without preserve_order: recursive key ordering is stable.
    let value = serde_json::to_value(value)
        .map_err(|_| error("SCHEMA_INVALID", "cannot serialize contract"))?;
    Ok(digest(&serde_json::to_vec(&value).map_err(|_| {
        error("SCHEMA_INVALID", "cannot encode contract")
    })?))
}
pub const SCHEMA: &str = include_str!("../schema/contracts.json");
static MANIFEST_SCHEMA: OnceLock<std::result::Result<jsonschema::Validator, String>> =
    OnceLock::new();

pub fn validate_manifest_value(value: &Value) -> Result<Manifest> {
    let validator = MANIFEST_SCHEMA
        .get_or_init(|| {
            let schema: Value = serde_json::from_str(SCHEMA).map_err(|e| e.to_string())?;
            jsonschema::validator_for(&schema).map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| error("SCHEMA_INVALID", e))?;
    if let Some(issue) = validator.iter_errors(value).next() {
        return Err(error(
            "SCHEMA_INVALID",
            format!("{}: {}", issue.instance_path, issue),
        ));
    }
    let manifest: Manifest = serde_json::from_value(value.clone())
        .map_err(|e| error("SCHEMA_INVALID", e.to_string()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}
pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest> {
    if bytes.len() > 1024 * 1024 {
        return Err(error("QUOTA_EXCEEDED", "manifest exceeds 1 MiB"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| error("SCHEMA_INVALID", "manifest must be UTF-8"))?;
    let data: toml::Value =
        toml::from_str(text).map_err(|e| error("SCHEMA_INVALID", e.to_string()))?;
    let value =
        serde_json::to_value(data).map_err(|_| error("SCHEMA_INVALID", "invalid TOML value"))?;
    validate_manifest_value(&value)
}
pub fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn safe_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value.contains(['\\', ':', '\0'])
        && !value.starts_with('/')
        && value.split('/').count() <= 16
        && value.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && !p.ends_with(['.', ' '])
                && !p.chars().any(char::is_control)
                && !matches!(
                    p.split('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_uppercase()
                        .as_str(),
                    "CON"
                        | "PRN"
                        | "AUX"
                        | "NUL"
                        | "COM1"
                        | "COM2"
                        | "COM3"
                        | "COM4"
                        | "COM5"
                        | "COM6"
                        | "COM7"
                        | "COM8"
                        | "COM9"
                        | "LPT1"
                        | "LPT2"
                        | "LPT3"
                        | "LPT4"
                        | "LPT5"
                        | "LPT6"
                        | "LPT7"
                        | "LPT8"
                        | "LPT9"
                )
        })
}
fn unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        if value.trim().is_empty() || !seen.insert(value) {
            return Err(error(
                "SCHEMA_INVALID",
                format!("duplicate/empty {label}: {value}"),
            ));
        }
    }
    Ok(())
}
pub fn validate_manifest(m: &Manifest) -> Result<()> {
    if m.schema_version != 1 {
        return Err(error("SCHEMA_UNSUPPORTED", "requires schema v1"));
    }
    if m.plugin.id.split('.').next() != Some(m.plugin.publisher.as_str())
        || m.plugin.publisher.is_empty()
    {
        return Err(error("SCHEMA_INVALID", "publisher namespace mismatch"));
    }
    let version = semver::Version::parse(&m.plugin.version)
        .map_err(|_| error("SCHEMA_INVALID", "invalid SemVer"))?;
    if !version.build.is_empty() {
        return Err(error(
            "SCHEMA_INVALID",
            "build metadata cannot distinguish releases",
        ));
    }
    semver::VersionReq::parse(&m.compatibility.platform_api)
        .map_err(|_| error("SCHEMA_INVALID", "invalid platform API range"))?;
    match m.compatibility.scope.as_str() {
        "portable" if m.compatibility.applications.is_empty() => (),
        "targeted" if !m.compatibility.applications.is_empty() => (),
        _ => {
            return Err(error(
                "SCHEMA_INVALID",
                "portable forbids application list; targeted requires one",
            ))
        }
    }
    unique(m.modules.iter().map(|v| v.id.as_str()), "module")?;
    unique(m.permissions.iter().map(|v| v.id.as_str()), "permission")?;
    unique(m.exports.iter().map(|v| v.id.as_str()), "export")?;
    unique(
        m.dependencies.iter().map(|v| v.plugin_id.as_str()),
        "dependency",
    )?;
    for module in &m.modules {
        for r in &module.requires {
            semver::VersionReq::parse(&r.version)
                .map_err(|_| error("SCHEMA_INVALID", "invalid capability range"))?;
        }
        for app in module
            .applications
            .iter()
            .chain(m.compatibility.applications.iter())
        {
            if let Some(v) = &app.version {
                semver::VersionReq::parse(v)
                    .map_err(|_| error("SCHEMA_INVALID", "invalid host range"))?;
            }
        }
        for permission in &module.permissions {
            if !m.permissions.iter().any(|p| &p.id == permission) {
                return Err(error(
                    "SCHEMA_INVALID",
                    "module references unknown permission",
                ));
            }
        }
        if module.r#type == "mcp-http/v1" {
            if module.endpoint.as_deref().is_none_or(str::is_empty) || module.entry.is_some() {
                return Err(error(
                    "SCHEMA_INVALID",
                    "HTTP MCP needs endpoint, not executable entry",
                ));
            }
        } else if !module.entry.as_deref().is_some_and(safe_path) {
            return Err(error("SCHEMA_INVALID", "module needs safe package entry"));
        }
        if module.r#type == "mcp-stdio/v1" && module.profile.is_none() {
            return Err(error(
                "SCHEMA_INVALID",
                "process isolation profile required",
            ));
        }
        if module.r#type == "wasm-component/v1"
            && module.world.as_deref() != Some("eos:plugin/component@1.0.0")
        {
            return Err(error("SCHEMA_INVALID", "unknown component world"));
        }
        for (k, v) in &module.env {
            if k.is_empty() || k.contains(['=', '\0']) || v.contains('\0') {
                return Err(error("SCHEMA_INVALID", "invalid process environment"));
            }
        }
        for c in &module.contributions {
            if c.id.is_empty() || c.id.contains(['/', '\\', '.']) {
                return Err(error("SCHEMA_INVALID", "invalid contribution ID"));
            }
        }
    }
    for dep in &m.dependencies {
        if dep.plugin_id == m.plugin.id {
            return Err(error("DEPENDENCY_CONFLICT", "self dependency"));
        }
        semver::VersionReq::parse(&dep.version)
            .map_err(|_| error("SCHEMA_INVALID", "invalid dependency range"))?;
        if !dep.required
            && !dep
                .module
                .as_ref()
                .is_some_and(|id| m.modules.iter().any(|v| &v.id == id && !v.required))
        {
            return Err(error(
                "SCHEMA_INVALID",
                "optional dependency must bind an optional module",
            ));
        }
    }
    for path in m
        .plugin
        .i18n
        .values()
        .chain(m.plugin.icon.iter())
        .chain(m.configuration.iter().map(|v| &v.schema))
    {
        if !safe_path(path) {
            return Err(error("SCHEMA_INVALID", "unsafe asset path"));
        }
    }
    if let Some(data) = &m.data {
        for migration in &data.migrations {
            if migration.to_version <= migration.from_version
                || migration.to_version > data.version
                || !m
                    .modules
                    .iter()
                    .any(|v| v.id == migration.module && v.r#type == "wasm-component/v1")
            {
                return Err(error(
                    "SCHEMA_INVALID",
                    "data migration must use a WASM module and advance its version",
                ));
            }
        }
        unique(
            data.migrations.iter().map(|v| v.module.as_str()),
            "migration module",
        )?;
    }
    for export in &m.exports {
        if !m.modules.iter().any(|v| v.id == export.module) {
            return Err(error("SCHEMA_INVALID", "export references unknown module"));
        }
        semver::Version::parse(&export.version)
            .map_err(|_| error("SCHEMA_INVALID", "invalid export version"))?;
        for method in &export.methods {
            if !safe_path(&method.input_schema) || !safe_path(&method.output_schema) {
                return Err(error("SCHEMA_INVALID", "unsafe export schema"));
            }
        }
    }
    Ok(())
}
pub fn configuration_validator(schema: &Value) -> Result<jsonschema::Validator> {
    fn local_refs(value: &Value) -> bool {
        match value {
            Value::Object(map) => map.iter().all(|(k, v)| {
                if k == "$ref" {
                    v.as_str().is_some_and(|s| s.starts_with('#'))
                } else {
                    local_refs(v)
                }
            }),
            Value::Array(items) => items.iter().all(local_refs),
            _ => true,
        }
    }
    if !schema.is_object() || !local_refs(schema) {
        return Err(error(
            "SCHEMA_INVALID",
            "configuration schema must be self-contained",
        ));
    }
    jsonschema::validator_for(schema)
        .map_err(|_| error("SCHEMA_INVALID", "invalid configuration schema"))
}
pub fn validate_configuration(schema: &Value, value: &Value) -> Result<()> {
    let validator = configuration_validator(schema)?;
    if let Some(issue) = validator.iter_errors(value).next() {
        return Err(error(
            "CONFIGURATION_INVALID",
            format!("{}: {}", issue.instance_path, issue),
        ));
    }
    Ok(())
}
pub fn io_error(error: std::io::Error) -> ErrorBody {
    let mut e = crate::error("IO_FAILED", "plugin storage operation failed");
    e.details = json!({"kind":format!("{:?}", error.kind())});
    e
}
