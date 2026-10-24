// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, Engine, Flags, SemanticProfile};
use std::sync::Arc;
fn output(
    pattern: &str,
    data: &[u8],
    insensitive: bool,
    profile: SemanticProfile,
) -> rfile::Result<Vec<u8>> {
    let literal = pattern
        .bytes()
        .map(|b| format!("\\x{b:02x}"))
        .collect::<String>();
    let source = format!(
        "0 regex/b{} {literal} value=%s\n",
        if insensitive { "c" } else { "" }
    );
    let mut engine = Engine::new(Arc::new(Database::from_source(&source).unwrap()));
    engine.flags = Flags(0xffb000);
    engine.profile = profile;
    Ok(engine.classify(data)?.output)
}
#[test]
fn corrected_histories_and_pinned_rejection_are_distinct() {
    for (pattern, input, expected) in [
        (r"(a|aa)+\1", b"aaa!".as_slice(), b"value=aaa".as_slice()),
        (r"(a*)+\1", b"a!", b"value=a"),
        (r"(a){0,2}\1", b"aa!", b"value=aa"),
    ] {
        assert_eq!(
            output(pattern, input, false, SemanticProfile::CorrectedV1).unwrap(),
            expected
        );
        assert!(matches!(
            output(pattern, input, false, SemanticProfile::Pinned548),
            Err(rfile::Error::Unsupported(_))
        ));
    }
}
#[test]
fn unknown_escape_icase_glibc_quirk_is_profiled() {
    assert_eq!(
        output(r"\d+", b"dDd!", true, SemanticProfile::Pinned548).unwrap(),
        b"data"
    );
    assert_eq!(
        output(r"\d+", b"dDd!", true, SemanticProfile::CorrectedV1).unwrap(),
        b"value=dDd"
    );
    assert_eq!(
        output(r"\D+", b"dDd!", true, SemanticProfile::Pinned548).unwrap(),
        b"value=dDd"
    );
}
#[test]
fn bracket_backslashes_ranges_and_newline_are_byte_predicates() {
    for (pattern, data, expected) in [
        (r"[\d]+", b"\\ddd!".as_slice(), b"value=\\ddd".as_slice()),
        (r"[^a]+", b"\nBB!", b"value=BB"),
        (r"[a&&b]+", b"a&&b!", b"value=a&&b"),
        (r"[]a]+", b"]aa!", b"value=]aa"),
    ] {
        assert_eq!(
            output(pattern, data, false, SemanticProfile::Pinned548).unwrap(),
            expected,
            "{pattern}"
        );
    }
}
#[test]
fn shared_database_keeps_profile_variants_and_capture_histories_isolated() {
    let db = Arc::new(Database::from_source("0 regex/bc \\\\d+ value=%s\n").unwrap());
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let db = db.clone();
            std::thread::spawn(move || {
                let mut e = Engine::new(db);
                e.flags = Flags(0xffb000);
                e.profile = if i % 2 == 0 {
                    SemanticProfile::Pinned548
                } else {
                    SemanticProfile::CorrectedV1
                };
                for _ in 0..50 {
                    assert_eq!(
                        e.classify(b"dDd!").unwrap().output,
                        if i % 2 == 0 {
                            b"data".as_slice()
                        } else {
                            b"value=dDd".as_slice()
                        }
                    );
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}
#[test]
fn invalid_ranges_groups_and_references_are_visible_source_errors() {
    for pattern in [
        r"[a--]+",
        r"[a-[:digit:]]",
        r"(a)\2",
        r"\1(a)",
        r"(a\1)",
        r"(?=a)",
        "(",
        "[[:unknown:]]",
    ] {
        let literal = pattern
            .bytes()
            .map(|b| format!("\\x{b:02x}"))
            .collect::<String>();
        assert!(
            Database::from_source(&format!("0 regex/b {literal} match=%s\n")).is_err(),
            "{pattern}"
        );
    }
}
#[test]
fn concurrent_calls_do_not_share_capture_slots() {
    let db = Arc::new(Database::from_source("0 regex/b ([ab])\\\\1 value=%s\n").unwrap());
    let workers: Vec<_> = (0..8)
        .map(|i| {
            let db = db.clone();
            std::thread::spawn(move || {
                let mut e = Engine::new(db);
                e.flags = Flags(0xffb000);
                let (data, expected) = if i % 2 == 0 {
                    (b"aa!", b"value=aa")
                } else {
                    (b"bb!", b"value=bb")
                };
                for _ in 0..100 {
                    assert_eq!(e.classify(data).unwrap().output, expected);
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}
