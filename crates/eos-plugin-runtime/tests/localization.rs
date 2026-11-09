use eos_plugin_runtime::{pack_directory, package_localizations, read_package};
use std::{fs, path::PathBuf};

#[test]
fn official_candidate_display_is_bound_to_unchanged_package_bytes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../official/meeting-notes");
    let bytes = pack_directory(&root).unwrap();
    let package = read_package(&bytes).unwrap();
    assert_eq!(package.manifest.plugin.version, "1.0.2");
    let manifest_before = serde_json::to_value(&package.manifest).unwrap();
    let digests = (
        package.digest.clone(),
        package.manifest_digest.clone(),
        package.files_digest.clone(),
    );
    let translations = package_localizations(&package).unwrap();
    assert_eq!(translations["en"].name, "EOS Meeting Notes");
    assert_eq!(translations["zh"].name, "EOS 会议纪要");
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
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../official/meeting-notes");
    let dir = tempfile::tempdir().unwrap();
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
    copy(&root, dir.path());
    fs::remove_file(dir.path().join("i18n/en.json")).unwrap();
    assert!(pack_directory(dir.path()).is_err());
}
