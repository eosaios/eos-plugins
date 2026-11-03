use eos_plugin_protocol::*;
const MANIFEST: &str = include_str!("../../../fixtures/portable-skills/plugin.toml");
#[test]
fn parses_confirmed_contract_and_rejects_ignored_fields() {
    let m = parse_manifest(MANIFEST.as_bytes()).unwrap();
    assert_eq!(
        (
            m.plugin.id.as_str(),
            m.compatibility.scope.as_str(),
            m.modules.len()
        ),
        ("eos.document-tools", "portable", 2)
    );
    for (a, b) in [
        ("schemaVersion = 1", "schemaVersion = 2"),
        ("required = true", "required = true\nunknown = 3"),
        ("publisher = \"eos\"", "publisher = \"other\""),
        ("permissions = []", "permissions = [\"missing\"]"),
    ] {
        let result = parse_manifest(MANIFEST.replace(a, b).as_bytes());
        assert!(result.is_err(), "{a} => {b}");
    }
}
#[test]
fn rejects_duplicate_keys_and_unsafe_cross_platform_paths() {
    assert!(
        parse_manifest(format!("{MANIFEST}\nschemaVersion = 1\nschemaVersion = 1").as_bytes())
            .is_err()
    );
    for path in [
        "../x",
        "/x",
        "C:/x",
        "a\\b",
        "x/../a",
        "a//b",
        "a/CON.txt",
        "a/file.",
        "a/aux",
    ] {
        assert!(!safe_path(path), "{path}");
    }
}
