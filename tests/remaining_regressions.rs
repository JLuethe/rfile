// SPDX-License-Identifier: GPL-3.0-only
use rfile::{Database, Engine, Flags};
use std::sync::Arc;

fn research() -> serde_json::Value {
    serde_json::from_str(include_str!("../reports/remaining-spec-observations.json")).unwrap()
}
fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn cdf() -> Vec<u8> {
    decode(research()["cdf_summary"]["fixture_hex"].as_str().unwrap())
}
fn regex(source: &str, input: &[u8]) -> Vec<u8> {
    Engine::new(Arc::new(Database::from_source(source).unwrap()))
        .classify(input)
        .unwrap()
        .output
}
#[test]
fn cdf_property_fixture_mime_matches_reference() {
    let mut e = Engine::builtin().unwrap();
    e.flags = Flags(Flags::MIME);
    assert_eq!(
        e.classify(&cdf()).unwrap().output,
        b"application/x-ole-storage; charset=binary"
    );
}
#[test]
fn regex_longest_and_collating_controls() {
    assert_eq!(
        regex("0 regex a|ab match=%s\n", b"ab\n"),
        b"match=ab, ASCII text"
    );
    assert_eq!(
        regex("0 regex [[.a.]] match=%s\n", b"a\n"),
        b"match=a, ASCII text"
    );
}
#[test]
fn cdf_summary_fallback_matches_pinned_reference() {
    assert_eq!(
        Engine::builtin().unwrap().classify(&cdf()).unwrap().output,
        b"Composite Document File V2 Document, Cannot read section info"
    );
}
#[test]
fn regex_backreference_matches_pinned_reference() {
    assert_eq!(
        regex("0 regex (ab)\\\\1 match=%s\n", b"abab\n"),
        b"match=abab, ASCII text"
    );
}
