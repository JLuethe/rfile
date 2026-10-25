// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, Engine, Flags, SemanticProfile};
use std::sync::Arc;
fn output(fmt: &str, value: f64, profile: SemanticProfile) -> rfile::Result<Vec<u8>> {
    let db = Database::from_source(&format!("0 ledouble x {fmt}\n")).unwrap();
    let db = Database::from_compiled(&db.compile().unwrap()).unwrap();
    let mut engine = Engine::new(Arc::new(db)).with_profile(profile);
    engine.flags = Flags(0xffb000);
    engine.classify(&value.to_le_bytes()).map(|r| r.output)
}
#[test]
fn precision_is_not_silently_clamped() {
    for p in [65, 128, 512] {
        assert_eq!(
            output(&format!("%.{p}f"), 1.25, SemanticProfile::Pinned548).unwrap(),
            format!("1.25{}", "0".repeat(p - 2)).as_bytes()
        );
    }
    assert_eq!(
        output("%.1022f", 1.0, SemanticProfile::Pinned548)
            .unwrap()
            .len(),
        1024
    );
    assert_eq!(
        output("%.1023f", 1.0, SemanticProfile::Pinned548)
            .unwrap_err()
            .to_string(),
        "Output buffer space exceeded 1025+0"
    );
}
#[test]
fn sign_zero_fill_alternate_and_nonfinite_contracts() {
    for (fmt, n, expected) in [
        ("%+010.2f", 1.25, "+000001.25"),
        ("%010.2f", -1.25, "-000001.25"),
        ("%010.2f", -0.0, "-000000.00"),
        ("%+010g", f64::INFINITY, "      +inf"),
        ("%010g", f64::NEG_INFINITY, "      -inf"),
        ("%#.0f", 1.25, "1."),
        ("%#.0e", 1.25, "1.e+00"),
        ("%#.1g", 1.25, "1."),
        ("%#.6g", 999999.9, "1.e+06"),
        ("%#.6g", 1e6, "1.00000e+06"),
        ("%.0f", 2.5, "2"),
        ("%.0f", 3.5, "4"),
    ] {
        assert_eq!(
            output(fmt, n, SemanticProfile::Pinned548).unwrap(),
            expected.as_bytes(),
            "{fmt} {n}"
        );
    }
}
#[test]
fn native_rejected_float_formats_are_profiled() {
    for fmt in ["%G", "%F", "%lf", "%hf", "%Lf"] {
        let e = output(fmt, 1.25, SemanticProfile::Pinned548).unwrap_err();
        assert_eq!(
            e.raw_diagnostic().unwrap(),
            format!("line 1: softmagic.c, 778: format `{fmt}' does not match with `%g'").as_bytes()
        );
        assert!(!e.is_limit());
    }
    assert_eq!(
        output("%G", f64::INFINITY, SemanticProfile::CorrectedV1).unwrap(),
        b"INF"
    );
    assert_eq!(
        output("%F", 1.25, SemanticProfile::CorrectedV1).unwrap(),
        b"1.250000"
    );
}
#[test]
fn native_field_limits_keep_diagnostic_quirks() {
    for (fmt, field) in [
        ("%1024f", "width"),
        ("%.1024f", "width"),
        ("%1.1024f", "precision"),
    ] {
        assert_eq!(
            output(fmt, 1.0, SemanticProfile::Pinned548)
                .unwrap_err()
                .to_string(),
            format!("Bad magic format `{fmt}' (field {field} too large: 1024)")
        );
    }
    assert!(Database::from_source("0 ledouble x %.4097f\n").is_err());
}
#[test]
fn integer_precision_and_character_padding_stay_spaces() {
    for (fmt, expected) in [("%08.3d", "     065"), ("%08c", "       A")] {
        let mut engine = Engine::new(Arc::new(
            Database::from_source(&format!("0 byte x {fmt}\n")).unwrap(),
        ));
        engine.flags = Flags(0xffb000);
        assert_eq!(
            engine.classify(b"ABCD").unwrap().output,
            expected.as_bytes()
        );
    }
}
