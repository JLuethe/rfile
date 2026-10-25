// SPDX-License-Identifier: GPL-3.0-only
use rfile::Database;
#[test]
fn listing_uses_first_description_and_mime_from_the_group() {
    let db=Database::from_source("0 string AB\n>2 byte 1 first\n!:mime application/x-first\n>3 byte 2 second\n!:mime application/x-second\n0 regex/t z+ text\n").unwrap();
    let bytes = db.legacy_list(4096).unwrap();
    assert!(
        bytes
            .windows(b"first [application/x-first]".len())
            .any(|s| s == b"first [application/x-first]")
    );
    assert!(!bytes.windows(6).any(|s| s == b"second"));
    assert!(
        bytes
            .windows(b"Text patterns:\n".len())
            .any(|s| s == b"Text patterns:\n")
    );
    let compiled = db.compile().unwrap();
    assert_eq!(
        bytes,
        Database::from_compiled(&compiled)
            .unwrap()
            .legacy_list(4096)
            .unwrap()
    );
    assert!(db.legacy_list(bytes.len() - 1).unwrap_err().is_limit());
    assert_eq!(db.legacy_list(bytes.len()).unwrap(), bytes);
}
#[test]
fn listing_keeps_byte_descriptions_and_honors_c_string_end() {
    let db = Database::from_source("0 string AB marker\n").unwrap();
    let mut wire = db.compile().unwrap();
    wire[432 + 160] = 255;
    wire[432 + 161] = 0;
    wire[432 + 162] = b'X';
    let bytes = Database::from_compiled(&wire)
        .unwrap()
        .legacy_list(4096)
        .unwrap();
    assert!(bytes.contains(&255));
    assert!(!bytes.contains(&b'X'));
    assert!(db.legacy_list(16 * 1024 * 1024 + 1).unwrap_err().is_limit());
}
