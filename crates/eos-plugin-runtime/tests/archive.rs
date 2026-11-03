use eos_plugin_runtime::read_package;
use std::io::Write;
fn archive(entries: &[(&str, u32, u8)]) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for (path, mode, kind) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(1);
        header.set_mode(*mode);
        header.set_entry_type(tar::EntryType::new(*kind));
        header.as_mut_bytes()[..100].fill(0);
        header.as_mut_bytes()[..path.len()].copy_from_slice(path.as_bytes());
        header.set_cksum();
        tar.append(&header, &b"x"[..]).unwrap();
    }
    let raw = tar.into_inner().unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&raw).unwrap();
    gz.finish().unwrap()
}
#[test]
fn rejects_escapes_links_modes_and_normalized_collisions() {
    for entries in [
        vec![("../escape", 0o644, b'0')],
        vec![("/absolute", 0o644, b'0')],
        vec![("C:/drive", 0o644, b'0')],
        vec![("sym", 0o644, b'2')],
        vec![("setuid", 0o4755, b'0')],
        vec![("A", 0o644, b'0'), ("a", 0o644, b'0')],
        vec![("ä", 0o644, b'0'), ("a\u{308}", 0o644, b'0')],
        vec![("x", 0o644, b'0'), ("x/child", 0o644, b'0')],
    ] {
        let failure = read_package(&archive(&entries))
            .err()
            .expect("unsafe archive rejected");
        assert_eq!(failure.code, "ARCHIVE_UNSAFE", "{entries:?}");
    }
}
