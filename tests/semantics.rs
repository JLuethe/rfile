// rfile contributions: Copyright (C) 2026 rfile contributors.
// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, Engine, Flags};
use std::{
    fs,
    io::{Cursor, Seek},
    sync::Arc,
};
fn engine(source: &str) -> Engine {
    Engine::new(Arc::new(Database::from_source(source).unwrap()))
}
fn classify(source: &str, data: &[u8]) -> String {
    String::from_utf8(engine(source).classify(data).unwrap().output).unwrap()
}
#[test]
fn primitives_and_endian() {
    assert_eq!(
        classify(
            "0 belong 0x12345678 big\n>4 leshort 0xabcd child-%x\n",
            &[0x12, 0x34, 0x56, 0x78, 0xcd, 0xab]
        ),
        "big child-ffffabcd"
    );
}
#[test]
fn masked_signed_and_unsigned() {
    assert_eq!(
        classify(
            "0 byte&0xf0 0xa0 masked\n>1 byte <0 negative\n>2 ubyte >127 unsigned\n",
            &[0xab, 0xff, 0xff]
        ),
        "masked negative unsigned"
    );
}
#[test]
fn failing_parent_skips_descendants() {
    assert_eq!(
        classify(
            "0 string XX parent\n>2 string bad never\n>>5 default x unreachable\n>2 string ok child\n",
            b"XXok.."
        ),
        "parent child"
    );
}
#[test]
fn default_clear() {
    assert_eq!(
        classify(
            "0 string AB root\n>2 byte 7 seven\n>2 default x fallback\n>2 clear x\n>2 default x cleared\n",
            b"AB\x07"
        ),
        "root seven cleared"
    );
}
#[test]
fn failed_negation_does_not_match() {
    assert_eq!(
        classify("0 string !AB wrong\n", b"AB\0"),
        "ASCII text, with no line terminators"
    );
}
#[test]
fn parent_relative_and_indirect() {
    assert_eq!(
        classify(
            "0 string AB root\n>&0 byte 4 value\n>(2.b) string XY indirect\n",
            b"AB\x04\0XY"
        ),
        "root value indirect"
    );
}
#[test]
fn named_base_and_flip() {
    assert_eq!(
        classify(
            "0 name n\n>0 leshort 0x1234 nested\n0 string AB root\n>2 use \\^n\n",
            &[b'A', b'B', 0x12, 0x34]
        ),
        "root nested"
    );
}
#[test]
fn strength_overrides() {
    assert_eq!(
        classify(
            "0 string ABC long\n0 string AB short\n!:strength +100\n",
            b"ABCD"
        ),
        "short"
    );
}
#[test]
fn nul_literal_and_nospace() {
    assert_eq!(
        classify(
            "0 string A\\0B type\n>3 byte 4 \\b, value=%d\n",
            b"A\0B\x04"
        ),
        "type, value=4"
    );
}
#[test]
fn annotations() {
    let mut e = engine("0 string AB type\n!:mime application/x-example\n!:ext ex/example\n");
    e.flags = Flags(Flags::MIME);
    assert_eq!(
        e.classify(b"ABcd").unwrap().output,
        b"application/x-example; charset=us-ascii"
    );
    e.flags = Flags(Flags::EXTENSION);
    assert_eq!(e.classify(b"ABcd").unwrap().output, b"ex/example");
}
#[test]
fn compiled_roundtrip_and_builtin() {
    let db = Database::builtin().unwrap();
    assert_eq!(db.rule_count(), 25176);
    let bytes = db.compile().unwrap();
    let other = Database::from_compiled(&bytes).unwrap();
    assert_eq!(other.rule_count(), db.rule_count());
    assert_eq!(other.compile().unwrap(), bytes);
}
#[test]
fn malformed_database() {
    let original = Database::builtin().unwrap().compile().unwrap();
    for n in [0, 1, 4, 16, 431, 433, original.len() - 1] {
        assert!(Database::from_compiled(&original[..n]).is_err());
    }
    let mut b = original.clone();
    b[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Database::from_compiled(&b).is_err());
    let mut b = original;
    b[432 + 6] = 255;
    assert!(Database::from_compiled(&b).is_err());
}
#[test]
fn recursion_and_work_are_errors() {
    let mut e = engine("0 name loop\n>0 use loop\n0 string AB\n>0 use loop\n");
    e.limits.names = 4;
    assert!(matches!(e.classify(b"ABcd"), Err(e) if e.is_limit()));
    let mut e = engine("0 string AB type\n");
    e.limits.steps = 0;
    assert!(matches!(e.classify(b"ABcd"), Err(e) if e.is_limit()));
}
#[test]
fn source_rejects_bad_trees_and_directives() {
    for s in [
        ">0 byte 1 nope\n",
        "0 unknown x nope\n",
        "!:mime application/nope\n",
        "0 byte 1 ok\n!:strength /0\n",
        "0 string A bad\n>>>2 byte 1 jump\n",
    ] {
        assert!(Database::from_source(s).is_err(), "{s}");
    }
}
#[test]
fn owned_results_and_threads() {
    let e = Arc::new(engine("0 string AB first\n0 string CD second\n"));
    let a = e.classify(b"ABcd").unwrap();
    assert_eq!(e.classify(b"CDab").unwrap().output, b"second");
    assert_eq!(a.output, b"first");
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let e = e.clone();
            std::thread::spawn(move || {
                for _ in 0..100 {
                    assert_eq!(e.classify(b"ABcd").unwrap().output, b"first");
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}
#[test]
fn stdin_reader_is_bounded() {
    let mut e = engine("0 string AB match\n");
    e.limits.bytes = 2;
    let mut reader = Cursor::new(b"ABCD");
    assert_eq!(e.classify_reader(&mut reader).unwrap().output, b"match");
    assert_eq!(reader.stream_position().unwrap(), 2);
}
#[test]
fn text_and_builtin_samples() {
    let e = Engine::builtin().unwrap();
    for (data, expected) in [
        (b"hi\n".as_slice(), "ASCII text"),
        (b"{\"a\":1}\n", "JSON text data"),
        (b"a,b\nc,d\n", "CSV ASCII text"),
        (b"x", "very short file (no magic)"),
        (b"", "empty"),
    ] {
        assert_eq!(e.classify(data).unwrap().output, expected.as_bytes());
    }
}
#[test]
fn file_position_is_restored() {
    let e = engine("0 string AB root\n");
    let path = std::env::temp_dir().join(format!("rfile-test-{}", std::process::id()));
    fs::write(&path, b"ABcd").unwrap();
    let mut f = fs::File::open(&path).unwrap();
    let _ = e.classify_file(&mut f).unwrap();
    assert_eq!(f.stream_position().unwrap(), 0);
    fs::remove_file(path).unwrap();
}
#[test]
fn regex_uses_longest_alternative_at_earliest_start() {
    assert_eq!(classify("0 regex a|ab [%s]\n", b"ab\n"), "[ab], ASCII text");
    assert_eq!(
        classify("0 regex a|abc$ [%s]\n", b"abcd\n"),
        "[a], ASCII text"
    );
    assert_eq!(
        classify("0 regex [^z]+ [%s]\n", b"a\nb\n"),
        "[a], ASCII text"
    );
}
#[test]
fn maximum_configured_recursion_is_bounded() {
    let mut e = engine("0 name loop\n>0 use loop\n0 string AB\n>0 use loop\n");
    e.limits.names = 150;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || assert!(matches!(e.classify(b"ABcd"), Err(e) if e.is_limit())))
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn raw_string_output_preserves_controls() {
    let mut e = engine("0 string x value:%s\n");
    assert_eq!(e.classify(b"A\x01B").unwrap().output, b"value:A\\001B");
    e.flags = Flags(Flags::RAW);
    assert_eq!(e.classify(b"A\x01B").unwrap().output, b"value:A\x01B");
}
#[test]
fn keep_going_keeps_pipeline_fallback() {
    let mut e = engine("0 string AB signature\n");
    e.flags = Flags(Flags::CONTINUE);
    assert_eq!(
        e.classify(b"AB\x01\0").unwrap().output,
        b"signature\\012- data"
    );
    e.flags = Flags(Flags::CONTINUE | Flags::MIME);
    assert_eq!(
        e.classify(b"AB\x01\0").unwrap().output,
        b"application/octet-stream; charset=binary"
    );
}
#[test]
fn source_database_boundaries_survive_merge() {
    let a = Database::from_source(
        "0 string AB one\n!:mime application/one\n0 string A two\n!:mime application/two\n",
    )
    .unwrap();
    let b = Database::from_source("0 string A three\n!:mime application/three\n").unwrap();
    let mut e = Engine::new(Arc::new(Database::merge(vec![a, b]).unwrap()));
    e.flags = Flags(Flags::CONTINUE | Flags::MIME_TYPE | Flags::NO_TEXT);
    assert_eq!(
        e.classify(b"AB\x01\0").unwrap().output,
        b"application/one\\012- application/three"
    );
}
#[test]
fn encoding_only_keep_going_preserves_single_separator_quirk() {
    let mut e = engine("0 string zzzz other\n");
    e.flags = Flags(Flags::CONTINUE | Flags::MIME_ENCODING);
    assert_eq!(
        e.classify(b"{\"a\":1}\n").unwrap().output,
        b"\\012- us-ascii"
    );
}

#[test]
fn padded_quads_and_raw_short_negation_follow_c_scratch_values() {
    let mut e = engine("0 lequad x value=%lld\n>&0 byte x child=%d\n");
    e.flags = Flags(0xffb000);
    assert_eq!(e.classify(&[1, 2]).unwrap().output, b"value=513");
    let mut e = engine("0 lequad !0 value=%lld\n");
    e.flags = Flags(0xffb000);
    assert_eq!(e.classify(&[0, 0]).unwrap().output, b"data");
    let mut e = engine("0 beshort&0xf0 !0 value=%d\n");
    e.flags = Flags(0xffb000);
    assert_eq!(
        e.classify(&[1]).unwrap().output,
        b"very short file (no magic)"
    );
    assert_eq!(e.classify(&[1, 0]).unwrap().output, b"data");
    let mut e = engine("0 belong&0xf0 !0 value=%d\n");
    e.flags = Flags(0xffb000);
    assert_eq!(e.classify(&[1, 2]).unwrap().output, b"value=513");
}

#[test]
fn odd_little_endian_string16_and_nonfinite_float_display() {
    let mut e = engine("0 lestring16 x value=%s\n");
    e.flags = Flags(0xffb000);
    assert_eq!(e.classify(b"A\0B").unwrap().output, b"value=AB");
    let mut e = engine("0 ledouble x value=%g\n");
    e.flags = Flags(0xffb000);
    assert_eq!(
        e.classify(&f64::from_bits(0xfff8_0000_0000_0000).to_le_bytes())
            .unwrap()
            .output,
        b"value=-nan"
    );
}
