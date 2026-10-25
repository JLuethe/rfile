// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, TimeContext};
#[test]
fn check_dump_preserves_parse_order_before_annotations_and_sorting() {
    let source = b"0 byte 1 small\n!:mime application/x-small\n0 string LONG longer\n";
    let report = Database::check_source_dump(source, "magic", &TimeContext::default(), 4096);
    assert!(report.database.is_ok());
    assert_eq!(report.output,b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\n*unknown*, 1: > 0 byte&,=1,\"small\"]\n*unknown*, 3: > 0 string,=LONG,\"longer\"]\n");
    let listed = report.database.unwrap().legacy_list(4096).unwrap();
    assert!(
        listed
            .windows(b"longer []\nSt".len())
            .any(|s| s == b"longer []\nSt")
    );
}
#[test]
fn malformed_and_bounded_checks_retain_only_completed_records() {
    let source = b"0 byte 1 good\n0 badtype 1 bad\n";
    let report = Database::check_source_dump(source, "magic", &TimeContext::default(), 4096);
    assert!(report.database.is_err());
    assert!(report.output.ends_with(b",\"good\"]\n"));
    let capped = Database::check_source_dump(source, "magic", &TimeContext::default(), 40);
    assert!(capped.database.unwrap_err().is_limit());
    assert_eq!(
        capped.output,
        b"cont\toffset\ttype\topcode\tmask\tvalue\tdesc\n"
    );
    let source = format!("0 pstring/L {} test\n", "A".repeat(127));
    assert!(
        Database::check_source_dump(source.as_bytes(), "magic", &TimeContext::default(), 4096)
            .database
            .is_err()
    );
}
