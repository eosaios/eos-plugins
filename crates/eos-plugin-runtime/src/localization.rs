//! Plain-text marketplace display metadata from declared package assets.
//! This projection never replaces or mutates the signed manifest.
use crate::Package;
use eos_plugin_protocol::{error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginDisplay {
    pub name: String,
    pub description: String,
}

/// Canonical lower-case language tags (for example en, zh, pt-br).
fn valid_locale(locale: &str) -> bool {
    if locale.len() > 35 {
        return false;
    }
    let mut parts = locale.split('-');
    let language = parts.next().unwrap_or_default();
    (2..=3).contains(&language.len())
        && language.bytes().all(|b| b.is_ascii_lowercase())
        && parts.all(|part| {
            !part.is_empty()
                && part.len() <= 8
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
fn valid_text(text: &str, limit: usize) -> bool {
    !text.trim().is_empty() && text.len() <= limit && !text.chars().any(|c| c.is_control())
}

/// Call only after package integrity/provenance has been checked by the caller.
/// Missing locales are an empty map, not invented translations.
pub fn package_localizations(package: &Package) -> Result<BTreeMap<String, PluginDisplay>> {
    let mut result = BTreeMap::new();
    for (locale, path) in &package.manifest.plugin.i18n {
        let bytes = package
            .files
            .get(path)
            .ok_or_else(|| error("SCHEMA_INVALID", "i18n asset missing"))?;
        // Existing UI resources may be non-JSON or larger. They remain unchanged;
        // only bounded JSON objects with our explicit namespace are projections.
        if bytes.len() > 1024 * 1024 {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
            continue;
        };
        if value.get("$eos.marketplace").is_none() {
            continue;
        }
        if !valid_locale(locale) {
            return Err(error("SCHEMA_INVALID", "noncanonical display locale"));
        }
        if bytes.len() > 16384 {
            return Err(error(
                "QUOTA_EXCEEDED",
                "marketplace display resource exceeds 16 KiB",
            ));
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "camelCase")]
        struct DisplayResource {
            schema_version: u64,
            name: String,
            description: String,
        }
        #[derive(Deserialize)]
        struct Resource {
            #[serde(rename = "$eos.marketplace")]
            display: DisplayResource,
        }
        // Reparse the original bytes, so duplicate namespace/display keys cannot
        // be hidden by serde_json::Value's last-key-wins object representation.
        let resource: Resource = serde_json::from_slice(bytes)
            .map_err(|_| error("SCHEMA_INVALID", "invalid marketplace display resource"))?;
        let display = resource.display;
        if display.schema_version != 1
            || !valid_text(&display.name, 256)
            || !valid_text(&display.description, 8192)
        {
            return Err(error("SCHEMA_INVALID", "invalid display text or version"));
        }
        result.insert(
            locale.clone(),
            PluginDisplay {
                name: display.name,
                description: display.description,
            },
        );
    }
    if result.len() > 32 {
        return Err(error("QUOTA_EXCEEDED", "at most 32 display locales"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn package() -> Package {
        Package {
            manifest: eos_plugin_protocol::parse_manifest(include_bytes!(
                "../../../fixtures/portable-skills/plugin.toml"
            ))
            .unwrap(),
            manifest_digest: "a".repeat(64),
            digest: "b".repeat(64),
            files_digest: "c".repeat(64),
            files: BTreeMap::new(),
            records: vec![],
        }
    }
    #[test]
    fn projects_without_changing_identity_and_allows_missing_translations() {
        let mut p = package();
        assert!(package_localizations(&p).unwrap().is_empty());
        p.manifest
            .plugin
            .i18n
            .insert("en".into(), "i18n/en.json".into());
        p.files.insert(
            "i18n/en.json".into(),
            br#"{"button.save":"Save","$eos.marketplace":{"schemaVersion":1,"name":"Notes","description":"Summarize supplied notes."}}"#.to_vec(),
        );
        let before = serde_json::to_value(&p.manifest).unwrap();
        assert_eq!(package_localizations(&p).unwrap()["en"].name, "Notes");
        assert_eq!(serde_json::to_value(&p.manifest).unwrap(), before);
        assert_eq!(p.manifest_digest, "a".repeat(64));
        assert_eq!(p.digest, "b".repeat(64));
    }
    #[test]
    fn legacy_ui_localization_is_not_reinterpreted() {
        let mut p = package();
        p.manifest
            .plugin
            .i18n
            .insert("en".into(), "ui/en.json".into());
        p.files
            .insert("ui/en.json".into(), br#"{"button.save":"Save"}"#.to_vec());
        assert!(package_localizations(&p).unwrap().is_empty());
        p.files.insert("ui/en.json".into(), b"save = Save".to_vec());
        assert!(package_localizations(&p).unwrap().is_empty());
        assert!(serde_json::to_value(&p.manifest).unwrap()["plugin"]
            .get("displayI18n")
            .is_none());
    }
    #[test]
    fn rejects_invalid_or_ambiguous_resources() {
        for body in [
            r#"{"$eos.marketplace":{"schemaVersion":1,"name":"Notes","description":"Text"},"$eos.marketplace":{"schemaVersion":1,"name":"Other","description":"Text"}}"#,
            r#"{"$eos.marketplace":{"schemaVersion":1,"name":"Notes","description":"Text","html":"x"}}"#,
            r#"{"$eos.marketplace":{"schemaVersion":1,"name":"Notes","name":"Other","description":"Text"}}"#,
            r#"{"$eos.marketplace":{"schemaVersion":1,"name":" ","description":"Text"}}"#,
            r#"{"$eos.marketplace":{"schemaVersion":1,"name":"Notes","description":"line\ncontrol"}}"#,
            r#"{"$eos.marketplace":{"schemaVersion":2,"name":"Notes","description":"Text"}}"#,
        ] {
            let mut p = package();
            p.manifest
                .plugin
                .i18n
                .insert("en".into(), "i18n/en.json".into());
            p.files
                .insert("i18n/en.json".into(), body.as_bytes().to_vec());
            assert!(package_localizations(&p).is_err(), "{body}");
        }
        let mut p = package();
        p.manifest
            .plugin
            .i18n
            .insert("en".into(), "missing.json".into());
        assert!(package_localizations(&p).is_err());
        p.files.insert("missing.json".into(), format!("{{\"$eos.marketplace\":{{\"schemaVersion\":1,\"name\":\"x\",\"description\":\"{}\"}}}}", "x".repeat(16385)).into_bytes());
        assert!(package_localizations(&p).is_err());
        for locale in ["EN", "en-US", "../en", "en--us", "x", "en_"] {
            assert!(!valid_locale(locale));
        }
        for locale in ["en", "zh", "pt-br", "zh-hant"] {
            assert!(valid_locale(locale));
        }
    }
}
