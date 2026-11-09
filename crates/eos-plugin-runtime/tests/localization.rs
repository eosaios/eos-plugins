use eos_plugin_runtime::{pack_directory, package_localizations, read_package};
use std::{fs, path::PathBuf};

fn localized_fixture() -> tempfile::TempDir {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let p = entry.unwrap().path();
            let target = to.join(p.file_name().unwrap());
            if p.is_dir() {
                copy(&p, &target);
            } else {
                fs::copy(p, target).unwrap();
            }
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/portable-skills");
    let dir = tempfile::tempdir().unwrap();
    copy(&root, dir.path());
    for (name, content) in [
        ("LICENSE", "Synthetic test fixture only"),
        ("README.md", "Synthetic display package"),
        ("skills/notes/SKILL.md", "# Synthetic notes"),
        ("ui/index.html", "<p>Synthetic fixture</p>"),
    ] {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    let manifest = dir.path().join("plugin.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[plugin.i18n]\nen = \"i18n/en.json\"\nzh = \"i18n/zh.json\"\n");
    fs::write(manifest, text).unwrap();
    fs::create_dir_all(dir.path().join("i18n")).unwrap();
    for (locale, name) in [("en", "Fixture Notes"), ("zh", "测试笔记")] {
        fs::write(dir.path().join(format!("i18n/{locale}.json")),serde_json::to_vec(&serde_json::json!({"$eos.marketplace":{"schemaVersion":1,"name":name,"description":"Synthetic display fixture"}})).unwrap()).unwrap();
    }
    dir
}

#[test]
fn display_is_bound_to_unchanged_package_bytes() {
    let dir = localized_fixture();
    let bytes = pack_directory(dir.path()).unwrap();
    let package = read_package(&bytes).unwrap();
    assert_eq!(package.manifest.plugin.version, "1.0.0");
    let manifest_before = serde_json::to_value(&package.manifest).unwrap();
    let digests = (
        package.digest.clone(),
        package.manifest_digest.clone(),
        package.files_digest.clone(),
    );
    let translations = package_localizations(&package).unwrap();
    assert_eq!(translations["en"].name, "Fixture Notes");
    assert_eq!(translations["zh"].name, "测试笔记");
    assert_eq!(
        serde_json::to_value(&package.manifest).unwrap(),
        manifest_before
    );
    assert_eq!(
        (
            package.digest,
            package.manifest_digest,
            package.files_digest
        ),
        digests
    );
}

#[test]
fn package_missing_declared_resource_is_rejected_before_display() {
    let dir = localized_fixture();
    fs::remove_file(dir.path().join("i18n/en.json")).unwrap();
    assert!(pack_directory(dir.path()).is_err());
}
